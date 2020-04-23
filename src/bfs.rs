use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use std::collections::HashMap;

use crate::dfs;

use dfs::graph::DfsGraph;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

fn materialize_path<N: Clone>(kns: &Vec<(usize, N)>, idx: usize) -> Vec<N> {
    let mut r = Vec::new();
    let mut idx = idx;
    loop {
        r.push(kns[idx].1.clone());

        if idx == 0 {
            break;
        }

        idx = kns[idx].0;
    }
    r.reverse();
    r
}

fn check_cycle<N: DfsNode>(kns: &Vec<(usize, N::KN)>, idx0: usize, kn: &N::KN) -> Option<(Vec<N::KN>, Vec<N::KN>)> {
    let mut idx = idx0;
    loop {
        if &kns[idx].1 == kn {
            break;
        }

        if idx == 0 {
            return None;
        }

        idx = kns[idx].0;
    }

    let mut cycle = Vec::new();
    let mut idx2 = idx0;
    loop {
        cycle.push(kns[idx2].1.clone());

        if idx2 == idx {
            break;
        }

        idx2 = kns[idx2].0;
    }
    cycle.reverse();

    let mut path = Vec::new();
    let mut idx2 = idx;
    loop {
        if idx2 == 0 {
            break;
        }

        idx2 = kns[idx2].0;
        path.push(kns[idx2].1.clone());
    }
    path.reverse();

    Some((path, cycle))
}

pub fn bfs<N: DfsNode, R: Send, GE: DfsGraph<N> + Sync, RE: DfsRes<N::KN, R> + Sync, LE: DfsLifecycle<N, R> + Sync>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let mut ql;
    let mut kns;

    if let Some(kn0) = n0.key_node() {
        ql = vec![(0, n0)];
        kns = vec![(0, kn0)];
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
                                            let mut path = materialize_path(&kns, prev_idx);
                                            path.push(kn2.clone());
                                            le.debug_end(&path);
                                            add_result(r, re.map_end(path));
                                            continue;
                                        }

                                        if let Some((path, cycle)) = check_cycle::<N>(&kns, prev_idx, kn2) {
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
                let prev_idx_new = kns.len();
                kns.push((prev_idx, kn));
                added = true;
                prev_idx = prev_idx_new;
            }
            (prev_idx, n)
        }).collect();

        eprintln!("Completed BFS step ql {} => q2 {}", ql.len(), q2.len());

        let ttl = kns.len();
        if added && ttl > 50_000_000 {
            let live_remap = rebuild_kns(&mut kns, q2.iter().map(|&(idx, _)| idx));
            q2 = q2.into_iter().map(|(idx, n)| (*live_remap.get(&idx).unwrap(), n)).collect();
            eprintln!("Rebuilt past size {} -> {}", ttl, kns.len());
        }
        else {
            eprintln!("Not rebuilding past size {}", ttl);
        }

        ql = q2;
        let firstest = match ql.first() {
            Some(&(idx, _)) => materialize_path(&kns, idx),
            None => vec![],
        };
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}

fn rebuild_kns<N>(ns: &mut Vec<(usize, N)>, live: impl Iterator<Item=usize>) -> HashMap<usize, usize> {
    let mut live: Vec<_> = live.collect();
    live.sort();
    live.dedup();
    live.reverse();
    let mut i = 0;
    while i < live.len() {
        let idx = live[i];
        if idx != 0 {
            let prev_idx = ns[idx].0;
            let last = *live.last().unwrap();
            if prev_idx < last {
                live.push(prev_idx);
            }
            else if prev_idx == last {
            }
            else {
                panic!();
            }
        }
        i += 1;
    }

    let mut live_remap = HashMap::new();
    let mut rebuilt_idx = 0;
    while let Some(idx) = live.pop() {
        assert!(rebuilt_idx <= idx, "{} <= {}?", rebuilt_idx, idx);
        ns.swap(rebuilt_idx, idx);
        // insert ourselves first so link from 0 to 0 can be looked up
        assert_eq!(idx == 0, rebuilt_idx == 0);
        assert!(!live_remap.contains_key(&idx));
        live_remap.insert(idx, rebuilt_idx);
        ns[rebuilt_idx].0 = *live_remap.get(&ns[rebuilt_idx].0).unwrap();
        rebuilt_idx += 1;
    }

    ns.truncate(rebuilt_idx);
    live_remap
}
