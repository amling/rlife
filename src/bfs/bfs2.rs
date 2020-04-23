use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use std::collections::VecDeque;

use crate::bfs;
use crate::dfs;

use bfs::kn_pile::KnPile;
use dfs::Path;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

pub fn bfs2<N: DfsNode, R, GE: DfsGraph<N> + Sync, RE: DfsRes<N::KN, R>, LE: DfsLifecycle<N, R>>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let mut kns;
    let mut q;
    let mut q_foresight;

    if let Some(kn0) = n0.key_node() {
        kns = KnPile::new(kn0);
        q = VecDeque::new();
        q.push_back((0, n0));
        q_foresight = 0;
    }
    else {
        panic!();
    }

    loop {
        let q_size = q.len();

        let mut r = re.empty();
        let add_result = |r: &mut R, r1| {
            let r0 = std::mem::replace(r, re.empty());
            *r = re.reduce(r0, r1);
        };

        // Step one: expand q into q2
        let mut q2 = VecDeque::new();
        let mut q2_foresight = q_foresight + 1;
        while let Some((prev_idx, n)) = q.pop_front() {
            for n2 in ge.expand(&n) {
                let kn2 = n2.key_node();
                if let Some(kn2) = &kn2 {
                    if ge.end(kn2) {
                        let mut path = kns.materialize_cloned(prev_idx);
                        path.push(kn2.clone());
                        le.debug_end(&path);
                        add_result(&mut r, re.map_end(path));
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
                        add_result(&mut r, re.map_cycle(path, cycle, kn2.clone()));
                        continue;
                    }
                }

                q2.push_back((prev_idx, n2, kn2));
            }

            compact(ge, le.threads(), &mut kns, &mut q, &mut q_foresight, &mut q2, &mut q2_foresight);
        }

        // Step two: fold q2 over into kns and q3
        let mut q3 = VecDeque::new();
        let mut q3_foresight = q2_foresight;
        while let Some((prev_idx, n, kn)) = q2.pop_front() {
            let mut prev_idx = prev_idx;
            if let Some(kn) = kn {
                prev_idx = kns.push(prev_idx, kn);
            }
            q3.push_back((prev_idx, n));

            compact(ge, le.threads(), &mut kns, &mut q3, &mut q3_foresight, &mut q2, &mut q2_foresight);
        }

        eprintln!("Completed BFS step {} => {}", q_size, q3.len());

        // start over
        q = q3;

        let firstest = match q.front() {
            Some(&(idx, _)) => kns.materialize_cloned(idx),
            None => vec![],
        };
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}

fn compact<N: DfsNode, GE: DfsGraph<N> + Sync>(ge: &GE, threads: usize, kns: &mut KnPile<N::KN>, qa: &mut VecDeque<(usize, N)>, qa_foresight: &mut usize, qb: &mut VecDeque<(usize, N, Option<N::KN>)>, qb_foresight: &mut usize) {
    // whatever kns thinks plus (usize, usize) for space during recompaction
    loop {
        let kns_size = kns.len() * (kns.esize() + std::mem::size_of::<(usize, usize)>());
        let qa_size = qa.len() * std::mem::size_of::<(usize, N)>();
        let qb_size = qb.len() * std::mem::size_of::<(usize, N, Option<N::KN>)>();
        if kns_size + qa_size + qb_size <= (1 << 33) {
            return;
        }

        deepen(ge, threads, "qa", qa, qa_foresight, |&(idx, ref n)| {
            let path = kns.materialize_cloned(idx);
            (Path::from_vec(path), n.clone())
        });
        deepen(ge, threads, "qb", qb, qb_foresight, |&(idx, ref n, ref kn)| {
            let mut path = kns.materialize_cloned(idx);
            if let Some(kn) = kn {
                path.push(kn.clone());
            }
            (Path::from_vec(path), n.clone())
        });

        let living = qa.iter().map(|&(idx, _)| idx).chain(qb.iter().map(|&(idx, _, _)| idx));
        let live_remap = kns.rebuild(living);
        for (idx, _) in qa.iter_mut() {
            *idx = *live_remap.get(idx).unwrap();
        }
        for (idx, _, _) in qb.iter_mut() {
            *idx = *live_remap.get(idx).unwrap();
        }
    }
}

fn deepen<T: Send, N: DfsNode, GE: DfsGraph<N> + Sync, F: Fn(&T) -> (Path<N>, N) + Send + Sync>(ge: &GE, threads: usize, name: &'static str, q: &mut VecDeque<T>, q_foresight: &mut usize, f: F) {
    let t0 = std::time::Instant::now();
    let size0 = q.len();
    let foresight0 = *q_foresight;
    let foresight1 = foresight0 + 1;
    eprintln!("Deepening {} from size {} foresight {}...", name, size0, foresight0);

    let shards = threads * 10;
    let mut q1s: Vec<_> = (0..shards).map(|i| {
        let ct = ((i + 1) * size0 / shards) - (i * size0 / shards);
        let q1: VecDeque<_> = q.drain(0..ct).collect();
        q1
    }).collect();

    {
        let wq = SegQueue::new();
        for q1 in q1s.iter_mut() {
            wq.push(q1);
        }

        crossbeam::scope(|sc| {
            for _ in 0..threads {
                sc.spawn(|_| {
                    loop {
                        let q1 = match wq.pop() {
                            Ok(q1) => q1,
                            Err(PopError) => {
                                return;
                            }
                        };

                        q1.retain(|t| {
                            let (mut path, n) = f(t);

                            deepen_search(ge, &mut path, n, foresight1)
                        });
                    }
                });
            }
        }).unwrap();
    }

    for mut q1 in q1s {
        q.append(&mut q1);
    }

    *q_foresight = foresight1;
    eprintln!("Deepened {} from size {} foresight {} to size {} foresight {} in {:?}", name, size0, foresight0, q.len(), foresight1, t0.elapsed());
}

fn deepen_search<N: DfsNode, GE: DfsGraph<N>>(ge: &GE, path: &mut Path<N>, n: N, foresight: usize) -> bool {
    if foresight == 0 {
        return true;
    }

    for n2 in ge.expand(&n) {
        let kn2 = n2.key_node();
        if let Some(kn2) = &kn2 {
            if ge.end(kn2) {
                return true;
            }

            if let Some(_) = path.find_or_push(kn2) {
                return true;
            }
        }

        let r = deepen_search(ge, path, n2, foresight - 1);

        if let Some(kn2) = &kn2 {
            path.pop(kn2);
        }

        if r {
            return true;
        }
    }

    false
}
