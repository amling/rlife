use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;

use crate::bfs;
use crate::dfs;

use bfs::kn_pile::KnPile;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

pub fn bfs1<N: DfsNode, R: Send, GE: DfsGraph<N> + Sync, RE: DfsRes<N::KN, R> + Sync, LE: DfsLifecycle<N, R> + Sync>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let mut kns;
    let mut ql;

    if let Some(kn0) = n0.key_node() {
        kns = KnPile::new(kn0);
        ql = vec![(0, n0)];
    }
    else {
        panic!();
    }

    loop {
        if ql.len() == 0 {
            break;
        }

        let threads = le.threads();
        let shards = threads * 10;

        let mut rs: Vec<_> = (0..shards).map(|_| re.empty()).collect();
        let mut q2s: Vec<_> = (0..shards).map(|_| Vec::new()).collect();

        {
            let q = SegQueue::new();
            for tuple in (0..shards).zip(rs.iter_mut()).zip(q2s.iter_mut()) {
                q.push(tuple);
            }

            crossbeam::scope(|sc| {
                for _ in 0..le.threads() {
                    sc.spawn(|_| {
                        loop {
                            let ((i, r), q2) = match q.pop() {
                                Result::Ok(tuple) => tuple,
                                Result::Err(PopError) => {
                                    return;
                                }
                            };

                            let add_result = |r: &mut R, r1| {
                                let r0 = std::mem::replace(r, re.empty());
                                *r = re.reduce(r0, r1);
                            };

                            let start = ql.len() * i / shards;
                            let end = ql.len() * (i + 1) / shards;
                            for idx in start..end {
                                let (prev_idx, ref n) = ql[idx];

                                for n2 in ge.expand(n) {
                                    let kn2 = n2.key_node();
                                    if let Some(kn2) = &kn2 {
                                        if ge.end(kn2) {
                                            let mut path = kns.materialize_cloned(prev_idx);
                                            path.push(kn2.clone());
                                            le.debug_end(&path);
                                            add_result(r, re.map_end(path));
                                            continue;
                                        }

                                        let hn2 = kn2.hash_node();
                                        let find_f = |idx, kn: &N::KN| {
                                            if kn.hash_node() == hn2 {
                                                Some(idx)
                                            }
                                            else {
                                                None
                                            }
                                        };
                                        if let Some(rep_idx) = kns.find(prev_idx, find_f) {
                                            let mut path = kns.materialize_cloned(rep_idx);
                                            path.pop().unwrap();
                                            let mut cycle = kns.materialize_cloned(idx);
                                            let path2: Vec<_> = cycle.drain(0..path.len()).collect();
                                            assert_eq!(path, path2);
                                            le.debug_cycle(&path, &cycle, kn2);
                                            add_result(r, re.map_cycle(path, cycle, kn2.clone()));
                                            continue;
                                        }
                                    }

                                    q2.push((prev_idx, n2, kn2));
                                }
                            }
                        }
                    });
                }
            }).unwrap();
        }

        let mut r = re.empty();
        for r1 in rs {
            r = re.reduce(r, r1);
        }

        let mut added = false;
        let mut q2: Vec<_> = q2s.into_iter().flatten().map(|(prev_idx, n, kn)| {
            let mut prev_idx = prev_idx;
            if let Some(kn) = kn {
                let prev_idx_new = kns.push(prev_idx, kn);
                added = true;
                prev_idx = prev_idx_new;
            }
            (prev_idx, n)
        }).collect();

        eprintln!("Completed BFS step ql {} => q2 {}", ql.len(), q2.len());

        let ttl = kns.len();
        if added && ttl > 50_000_000 {
            let live_remap = kns.rebuild(q2.iter().map(|&(idx, _)| idx));
            q2 = q2.into_iter().map(|(idx, n)| (*live_remap.get(&idx).unwrap(), n)).collect();
            eprintln!("Rebuilt past size {} -> {}", ttl, kns.len());
        }
        else {
            eprintln!("Not rebuilding past size {}", ttl);
        }

        ql = q2;
        let firstest = match ql.first() {
            Some(&(idx, _)) => kns.materialize_cloned(idx),
            None => vec![],
        };
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}
