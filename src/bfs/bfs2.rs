#![allow(unused_parens)]

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
    let mut qa;
    let mut qa_foresight;
    let mut q0 = VecDeque::new();

    if let Some(kn0) = n0.key_node() {
        kns = KnPile::new(kn0);
        qa = VecDeque::new();
        qa.push_back((0, n0));
        qa_foresight = 0;
    }
    else {
        panic!();
    }

    loop {
        let qa_size = qa.len();

        if qa_size == 0 {
            break;
        }

        let mut r = re.empty();
        let add_result = |r: &mut R, r1| {
            let r0 = std::mem::replace(r, re.empty());
            *r = re.reduce(r0, r1);
        };

        // Step one: expand qa into qb
        let mut qb = VecDeque::new();
        while let Some((prev_idx, n)) = qa.pop_front() {
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

                qb.push_back((prev_idx, n2, kn2));
            }

            compact(ge, le.threads(), &mut kns, &mut qa_foresight, &mut qa, &mut qb, &mut q0);
        }
        // should be unused after this
        drop(qa);

        // Step two: fold qb over into kns and qc
        let mut qc = VecDeque::new();
        while let Some((prev_idx, n, kn)) = qb.pop_front() {
            let mut prev_idx = prev_idx;
            if let Some(kn) = kn {
                prev_idx = kns.push(prev_idx, kn);
            }
            qc.push_back((prev_idx, n));

            compact(ge, le.threads(), &mut kns, &mut qa_foresight, &mut q0, &mut qb, &mut qc);
        }
        drop(qb);

        // start over
        qa = qc;
        qa_foresight = match qa_foresight {
            0 => 0,
            _ => qa_foresight - 1,
        };

        eprintln!("Completed BFS step {} => {}, estimated memory {}", qa_size, qa.len(), fmt_mem(kns_mem(&kns) + vd_mem(&qa)));

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

fn kns_mem<N>(kns: &KnPile<N>) -> usize {
    // whatever kns thinks plus (usize, usize) for space during recompaction
    kns.len() * (kns.esize() + std::mem::size_of::<(usize, usize)>())
}

fn vd_mem<T>(q: &VecDeque<T>) -> usize {
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

fn compact<N: DfsNode, GE: DfsGraph<N> + Sync>(ge: &GE, threads: usize, kns: &mut KnPile<N::KN>, qa_foresight: &mut usize, qa: &mut VecDeque<(usize, N)>, qb: &mut VecDeque<(usize, N, Option<N::KN>)>, qc: &mut VecDeque<(usize, N)>) {
    loop {
        let mem = kns_mem(kns) + vd_mem(qa) + vd_mem(qb) + vd_mem(qc);
        if kns_mem(kns) + vd_mem(qa) + vd_mem(qb) + vd_mem(qc) <= (1 << 34) {
            return;
        }
        eprintln!("Estimated memory {}, deepening...", fmt_mem(mem));

        *qa_foresight += 1;
        deepen(ge, threads, "qa", qa, *qa_foresight, |&(idx, ref n)| {
            let path = kns.materialize_cloned(idx);
            (Path::from_vec(path), n.clone())
        });
        deepen(ge, threads, "qb", qb, *qa_foresight - 1, |&(idx, ref n, ref kn)| {
            let mut path = kns.materialize_cloned(idx);
            if let Some(kn) = kn {
                path.push(kn.clone());
            }
            (Path::from_vec(path), n.clone())
        });
        deepen(ge, threads, "qc", qc, *qa_foresight - 1, |&(idx, ref n)| {
            let path = kns.materialize_cloned(idx);
            (Path::from_vec(path), n.clone())
        });

        let living = vec![].into_iter();
        let living = living.chain(qa.iter().map(|&(idx, _)| idx));
        let living = living.chain(qb.iter().map(|&(idx, _, _)| idx));
        let living = living.chain(qc.iter().map(|&(idx, _)| idx));
        let live_remap = kns.rebuild(living);
        for (idx, _) in qa.iter_mut() {
            *idx = *live_remap.get(idx).unwrap();
        }
        for (idx, _, _) in qb.iter_mut() {
            *idx = *live_remap.get(idx).unwrap();
        }
        for (idx, _) in qc.iter_mut() {
            *idx = *live_remap.get(idx).unwrap();
        }
    }
}

fn deepen<T: Send, N: DfsNode, GE: DfsGraph<N> + Sync, F: Fn(&T) -> (Path<N>, N) + Send + Sync>(ge: &GE, threads: usize, name: &'static str, q: &mut VecDeque<T>, foresight: usize, f: F) {
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
