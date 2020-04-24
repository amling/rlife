#![allow(unused_parens)]

use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;

use crate::bfs;
use crate::dfs;

use bfs::chunk_queue::ChunkQueue;
use bfs::kn_pile::KnPile;
use dfs::Path;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

pub fn bfs2<N: DfsNode, R: Send, GE: DfsGraph<N> + Sync, RE: DfsRes<N::KN, R> + Sync, LE: DfsLifecycle<N, R> + Sync>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let threads = le.threads();
    let shards = threads * 10;

    let mut kns;
    let mut qa;
    let mut qa_foresight;
    let mut depth = 0;

    if let Some(kn0) = n0.key_node() {
        kns = KnPile::new(kn0);
        qa = ChunkQueue::new();
        qa.push_back((0, n0));
        qa_foresight = 0;
    }
    else {
        panic!();
    }

    loop {
        // space each element of qa may expand into, either in qb or in kns and qc
        // TODO: this 2 expansion factor is a total hack
        let per_element_space = 2 * (std::mem::size_of::<(usize, N, Option<N::KN>)>().max(kns.esize() + std::mem::size_of::<(usize, N)>()));

        loop {
            let mem = kns.len() * kns_el_mem(&kns) + per_element_space * qa.len();
            if mem <= (1 << 34) {
                eprintln!("Estimated required memory {}, expanding...", fmt_mem(mem));
                break;
            }
            eprintln!("Estimated required memory {}, deepening...", fmt_mem(mem));

            qa_foresight += 1;
            deepen(ge, threads, "qa", &mut qa, qa_foresight, |&(idx, ref n)| {
                let path = kns.materialize_cloned(idx);
                (Path::from_vec(path), n.clone())
            });

            let living = vec![].into_iter();
            let living = living.chain(qa.iter().map(|&(idx, _)| idx));
            let live_remap = kns.rebuild(living);

            let t0 = std::time::Instant::now();
            {
                let wq = SegQueue::new();
                for chunk in qa.chunks_mut() {
                    wq.push(chunk);
                }

                crossbeam::scope(|sc| {
                    for _ in 0..threads {
                        sc.spawn(|_| {
                            loop {
                                let chunk = match wq.pop() {
                                    Ok(chunk) => chunk,
                                    Err(PopError) => {
                                        return;
                                    }
                                };

                                for (idx, _) in chunk {
                                    *idx = *live_remap.get(idx).unwrap();
                                }
                            }
                        });
                    }
                }).unwrap();
            }
            eprintln!("Reindexed qa in {:?}", t0.elapsed());
        }

        let qa_size = qa.len();

        if qa_size == 0 {
            break;
        }

        let add_result = |r: &mut R, r1| {
            let r0 = std::mem::replace(r, re.empty());
            *r = re.reduce(r0, r1);
        };

        // Step one: expand qa into qb
        let mut qa1s = qa.drain_partition(shards);
        let mut qb1s: Vec<_> = qa1s.iter().map(|_| ChunkQueue::new()).collect();
        let mut r1s: Vec<_> = qa1s.iter().map(|_| re.empty()).collect();

        {
            let wq = SegQueue::new();
            for tuple in qa1s.iter_mut().zip(qb1s.iter_mut()).zip(r1s.iter_mut()) {
                wq.push(tuple);
            }

            crossbeam::scope(|sc| {
                for _ in 0..threads {
                    sc.spawn(|_| {
                        loop {
                            let ((qa1, qb1), r1) = match wq.pop() {
                                Ok(tuple) => tuple,
                                Err(PopError) => {
                                    return;
                                }
                            };

                            while let Some((prev_idx, n)) = qa1.pop_front() {
                                for n2 in ge.expand(&n) {
                                    let kn2 = n2.key_node();
                                    if let Some(kn2) = &kn2 {
                                        if ge.end(kn2) {
                                            let mut path = kns.materialize_cloned(prev_idx);
                                            path.push(kn2.clone());
                                            le.debug_end(&path);
                                            add_result(r1, re.map_end(path));
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
                                            add_result(r1, re.map_cycle(path, cycle, kn2.clone()));
                                            continue;
                                        }
                                    }

                                    qb1.push_back((prev_idx, n2, kn2));
                                }
                            }
                        }
                    });
                }
            }).unwrap();
        }

        let mut r = re.empty();
        for r1 in r1s {
            r = re.reduce(r, r1);
        }

        let mut qb = ChunkQueue::new();
        for mut qb1 in qb1s {
            qb.append(&mut qb1);
        }

        // Step two: fold qb over into kns and qc
        let mut qc = ChunkQueue::new();
        while let Some((prev_idx, n, kn)) = qb.pop_front() {
            let mut prev_idx = prev_idx;
            if let Some(kn) = kn {
                prev_idx = kns.push(prev_idx, kn);
            }
            qc.push_back((prev_idx, n));
        }

        // start over
        qa = qc;
        qa_foresight = match qa_foresight {
            0 => 0,
            _ => qa_foresight - 1,
        };
        depth += 1;

        eprintln!("Completed BFS step to depth {}, size {} => {}, estimated memory {}", depth, qa_size, qa.len(), fmt_mem(kns_mem(&kns) + q_mem(&qa)));

        let firstest = match qa.front() {
            Some(&(idx, _)) => kns.materialize_cloned(idx),
            None => vec![],
        };
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}

fn kns_el_mem<N>(kns: &KnPile<N>) -> usize {
    // whatever kns thinks plus (usize, usize) for space during recompaction
    kns.esize() + std::mem::size_of::<(usize, usize)>()
}

fn kns_mem<N>(kns: &KnPile<N>) -> usize {
    kns.len() * kns_el_mem(kns)
}

fn q_mem<T>(q: &ChunkQueue<T>) -> usize {
    q.len() * std::mem::size_of::<T>()
}

fn fmt_mem(mem: usize) -> String {
    let g = (1 << 30);
    if mem >= g {
        return format!("{:.2} GB", (mem as f64) / (g as f64));
    }

    let m = (1 << 20);
    if mem >= m {
        return format!("{:.2} MB", (mem as f64) / (m as f64));
    }

    let k = (1 << 20);
    if mem >= k {
        return format!("{:.2} KB", (mem as f64) / (k as f64));
    }

    return format!("{} B", mem);
}

fn deepen<T: Send, N: DfsNode, GE: DfsGraph<N> + Sync, F: Fn(&T) -> (Path<N>, N) + Send + Sync>(ge: &GE, threads: usize, name: &'static str, q: &mut ChunkQueue<T>, foresight: usize, f: F) {
    if foresight == 0 {
        return;
    }
    let size0 = q.len();
    if size0 == 0 {
        return;
    }

    let t0 = std::time::Instant::now();
    eprintln!("Deepening {} from size {}...", name, size0);

    let shards = threads * 10;
    let mut q1s = q.drain_partition(shards);

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

                            deepen_search(ge, &mut path, n, foresight)
                        });
                    }
                });
            }
        }).unwrap();
    }

    for mut q1 in q1s {
        q.append(&mut q1);
    }

    eprintln!("Deepened {} from size {} to size {} foresight {} in {:?}", name, size0, q.len(), foresight, t0.elapsed());
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
