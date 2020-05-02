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

pub fn bfs1<N: DfsNode, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N> + Sync>(n0: N, ge: &GE, le: &mut LE) {
    let mut kns;
    let mut ql;

    if let Some(kn0) = n0.key_node() {
        kns = KnPile::of(vec![kn0]);
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

        let mut rs: Vec<_> = (0..shards).map(|_| DfsRes::new()).collect();
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

                            let start = ql.len() * i / shards;
                            let end = ql.len() * (i + 1) / shards;
                            for idx in start..end {
                                let (prev_idx, ref n) = ql[idx];

                                for n2 in ge.expand(n) {
                                    let kn2 = n2.key_node();
                                    if let Some(kn2) = &kn2 {
                                        if let Some(label) = ge.end(kn2) {
                                            let mut path = kns.materialize_cloned(prev_idx);
                                            path.push(kn2.clone());
                                            le.debug_end(&path, label);
                                            r.add_end(path, label);
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
                                            let mut cycle = kns.materialize_cloned(prev_idx);
                                            let path2: Vec<_> = cycle.drain(0..path.len()).collect();
                                            assert_eq!(path, path2);
                                            le.debug_cycle(&path, &cycle, kn2);
                                            r.add_cycle(path, cycle, kn2.clone());
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

        let mut r = DfsRes::new();
        for mut r1 in rs {
            r.append(&mut r1);
        }

        let mut added = false;
        let mut q2: Vec<_> = q2s.into_iter().flatten().map(|(prev_idx, n, kn)| {
            let mut prev_idx = prev_idx;
            if let Some(kn) = kn {
                prev_idx = kns.push(prev_idx, kn);
                added = true;
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
        if let Some(&(idx, ref n)) = ql.first() {
            le.on_recollect_firstest((kns.materialize_cloned(idx), n.clone()));
        }
        if !le.on_recollect_results(r) {
            break;
        }
    }
}
