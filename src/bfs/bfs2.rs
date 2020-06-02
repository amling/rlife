#![allow(unused_parens)]

use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use serde::Deserialize;
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::bfs;
use crate::dfs;

use bfs::chunk_queue::ChunkQueue;
use bfs::kn_pile::KnPile;
use dfs::Path;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use dfs::res::DfsRes;

struct WorkUnit<N: DfsNode> {
    q: ChunkQueue<(usize, N)>,
    q2: ChunkQueue<(usize, N)>,
    r: DfsRes<N::KN>,
}

impl<N: DfsNode> WorkUnit<N> {
    fn new(q: ChunkQueue<(usize, N)>) -> Self {
        WorkUnit {
            q: q,
            q2: ChunkQueue::new(),
            r: DfsRes::new(),
        }
    }
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct Bfs2State<N, KN: Default> {
    kns: KnPile<KN>,
    q: ChunkQueue<(usize, N)>,
    foresight: usize,
    depth: usize,
}

impl<N: DfsNode> Bfs2State<N, N::KN> {
    pub fn new(init: impl IntoIterator<Item=(Vec<N::KN>, N)>) -> Bfs2State<N, N::KN> {
        let mut kns = KnPile::new();
        let mut q = ChunkQueue::new();

        for (kn0s, n0) in init.into_iter() {
            let mut idx = 0;
            for kn0 in kn0s {
                idx = kns.push(idx, kn0);
            }
            q.push_back((idx, n0));
        }

        Bfs2State {
            kns: kns,
            q: q,
            foresight: 0,
            depth: 0,
        }
    }
}

pub fn bfs2<N: DfsNode, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N> + Sync>(mut state: Bfs2State<N, N::KN>, ge: &GE, le: &mut LE) {
    loop {
        let threads = le.threads();
        let shards = threads * 10;

        le.debug_bfs2_checkpoint(|le| {
            let kns = &mut state.kns;
            let q = &mut state.q;

            let living = vec![].into_iter();
            let living = living.chain(q.iter().map(|&(idx, _)| idx));
            let live_remap = kns.rebuild(living, |msg| le.log(LogLevel::INFO, msg));

            {
                let t0 = std::time::Instant::now();
                cq_par(threads, shards, q, |q| {
                    for (idx, _) in q.iter_mut() {
                        *idx = *live_remap.get(idx).unwrap();
                    }
                });
                le.log(LogLevel::INFO, format!("Reindexed q in {:?}", t0.elapsed()));
            }

            &state
        });

        let kns = &mut state.kns;
        let q = &mut state.q;
        let foresight = &mut state.foresight;
        let depth = &mut state.depth;

        let size = q.len();

        if size == 0 {
            break;
        }

        {
            let ql = q.len();
            let qm = q_mem(&q);
            let kl = kns.len();
            let km = kns_mem(&kns);
            let m = qm + km;
            le.log(LogLevel::INFO, format!("q {} ({}), kns {} ({}), total {}", ql, fmt_mem(qm), kl, fmt_mem(km), fmt_mem(m)));
        }

        // step1: split q into work units
        let mut ws: Vec<_> = q.drain_partition(shards).into_iter().map(|q| WorkUnit::new(q)).collect();

        // step2: expand until all work units are done, deepening as needed
        loop {
            let stop = AtomicBool::new(false);

            // step 2a: expand as much as we can until/unless we go over memory
            let mem = ws.iter().map(|w| q_mem(&w.q) + q_mem(&w.q2)).sum::<usize>() + kns_mem(&kns);
            let mem = AtomicUsize::new(mem);
            let mem_max = le.max_mem();

            singleton_par(threads, &mut ws, |w| {
                loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }

                    let wm0 = q_mem(&w.q) + q_mem(&w.q2);
                    let (prev_idx, n) = match w.q.pop_front() {
                        Some(p) => p,
                        None => {
                            break;
                        }
                    };

                    for n2 in ge.expand(&n, kns.path_iter(prev_idx)) {
                        let kn2 = n2.key_node();
                        if let Some(kn2) = &kn2 {
                            if let Some(label) = ge.end(kn2, kns.path_iter(prev_idx)) {
                                let mut path = kns.materialize_cloned(prev_idx);
                                path.push(kn2.clone());
                                le.debug_end(&path, label);
                                w.r.add_end(path, label);
                                continue;
                            }

                            let hn2 = kn2.hash_node(kns.path_iter(prev_idx));
                            let find_f = |idx, prev_idx, kn: &N::KN| {
                                if kn.hash_node(kns.path_iter(prev_idx)) == hn2 {
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
                                w.r.add_cycle(path, cycle, kn2.clone());
                                continue;
                            }
                        }

                        w.q2.push_back((prev_idx, n2));
                    }

                    let wm1 = q_mem(&w.q) + q_mem(&w.q2);

                    if wm1 >= wm0 {
                        let inc = wm1 - wm0;
                        let old = mem.fetch_add(inc, Ordering::Relaxed);
                        if old + inc > mem_max {
                            // we're out of memory, bail
                            stop.store(true, Ordering::Relaxed);
                            return;
                        }
                    }
                    else {
                        mem.fetch_sub(wm0 - wm1, Ordering::Relaxed);
                    }
                }
            });

            if stop.load(Ordering::Relaxed) {
                // step2b: deepen (as needed)
                let m_before = {
                    let ql = ws.iter().map(|w| w.q.len()).sum::<usize>();
                    let qm = ws.iter().map(|w| q_mem(&w.q)).sum::<usize>();
                    let q2l = ws.iter().map(|w| w.q2.len()).sum::<usize>();
                    let q2m = ws.iter().map(|w| q_mem(&w.q2)).sum::<usize>();
                    let kl = kns.len();
                    let km = kns_mem(&kns);
                    let m = qm + q2m + km;
                    le.log(LogLevel::INFO, format!("ws.q {} ({}), ws.q2 {} ({}), kns {} ({}), total {}, deepening...", ql, fmt_mem(qm), q2l, fmt_mem(q2m), kl, fmt_mem(km), fmt_mem(m)));

                    m
                };
                let t0 = std::time::Instant::now();

                *foresight += 1;

                {
                    let before = ws.iter().map(|w| w.q.len()).sum::<usize>();
                    let t0 = std::time::Instant::now();
                    singleton_par(threads, &mut ws, |w| {
                        w.q.retain(|&(prev_idx, ref n)| {
                            let path = kns.materialize_cloned(prev_idx);
                            let mut path = Path::from_vec(path);
                            let n = n.clone();

                            deepen_search(ge, &mut path, n, *foresight)
                        });
                    });
                    let after = ws.iter().map(|w| w.q.len()).sum::<usize>();
                    le.log(LogLevel::INFO, format!("Deepened ws.q {} => {}, foresight {} in {:?}", before, after, *foresight, t0.elapsed()));

                    let t0 = std::time::Instant::now();
                    singleton_par(threads, &mut ws, |w| {
                        w.q.defragment();
                    });
                    le.log(LogLevel::INFO, format!("Defragmented ws.q in {:?}", t0.elapsed()));
                }

                {
                    let before = ws.iter().map(|w| w.q2.len()).sum::<usize>();
                    let t0 = std::time::Instant::now();
                    singleton_par(threads, &mut ws, |w| {
                        w.q2.retain(|&(prev_idx, ref n)| {
                            let mut path = kns.materialize_cloned(prev_idx);
                            if let Some(kn) = n.key_node() {
                                path.push(kn.clone());
                            }
                            let mut path = Path::from_vec(path);
                            let n = n.clone();

                            deepen_search(ge, &mut path, n, *foresight - 1)
                        });
                    });
                    let after = ws.iter().map(|w| w.q2.len()).sum::<usize>();
                    le.log(LogLevel::INFO, format!("Deepened ws.q2 {} => {}, foresight {} in {:?}", before, after, *foresight - 1, t0.elapsed()));

                    let t0 = std::time::Instant::now();
                    singleton_par(threads, &mut ws, |w| {
                        w.q2.defragment();
                    });
                    le.log(LogLevel::INFO, format!("Defragmented ws.q2 in {:?}", t0.elapsed()));
                }

                let living = vec![].into_iter();
                let living = living.chain(ws.iter().map(|w| w.q.iter().map(|&(idx, _)| idx)).flatten());
                let living = living.chain(ws.iter().map(|w| w.q2.iter().map(|&(idx, _)| idx)).flatten());
                let live_remap = kns.rebuild(living, |msg| le.log(LogLevel::INFO, msg));

                {
                    let t0 = std::time::Instant::now();
                    singleton_par(threads, &mut ws, |w| {
                        for (idx, _) in w.q.iter_mut() {
                            *idx = *live_remap.get(idx).unwrap();
                        }
                    });
                    le.log(LogLevel::INFO, format!("Reindexed ws.q in {:?}", t0.elapsed()));
                }

                {
                    let t0 = std::time::Instant::now();
                    singleton_par(threads, &mut ws, |w| {
                        for (idx, _) in w.q2.iter_mut() {
                            *idx = *live_remap.get(idx).unwrap();
                        }
                    });
                    le.log(LogLevel::INFO, format!("Reindexed ws.q2 in {:?}", t0.elapsed()));
                }

                {
                    let qm = ws.iter().map(|w| q_mem(&w.q)).sum::<usize>();
                    let q2m = ws.iter().map(|w| q_mem(&w.q2)).sum::<usize>();
                    let km = kns_mem(&kns);
                    let m = qm + q2m + km;
                    le.log(LogLevel::INFO, format!("Deepening pass {} => {} completed in {:?}", fmt_mem(m_before), fmt_mem(m), t0.elapsed()));
                }
            }
            else {
                // finished, great
                break;
            }
        }

        // step3a: fold work unit results together
        let mut q3 = ChunkQueue::new();
        let mut r = DfsRes::new();
        for mut w in ws {
            assert_eq!(0, w.q.len());

            q3.append(&mut w.q2);
            r.append(&mut w.r);
        }

        // step3b: fold results into kns
        let mut mem_max = le.max_mem();
        let mut q4 = ChunkQueue::new();
        while let Some((prev_idx, n)) = q3.pop_front() {
            let mut prev_idx = prev_idx;
            if let Some(kn) = n.key_node() {
                prev_idx = kns.push(prev_idx, kn);
            }
            q4.push_back((prev_idx, n));

            loop {
                // step3b.1: deepen (while needed)
                let m_before = {
                    let q3l = q3.len();
                    let q3m = q_mem(&q3);
                    let q4l = q4.len();
                    let q4m = q_mem(&q4);
                    let kl = kns.len();
                    let km = kns_mem(&kns);
                    let m = q3m + q4m + km;

                    if m <= mem_max {
                        break;
                    }

                    // let's reread up to once per deepen to see if it's changed
                    mem_max = le.max_mem();
                    if m <= mem_max {
                        break;
                    }

                    le.log(LogLevel::INFO, format!("q3 {} ({}), q4 {} ({}), kns {} ({}), total {}, deepening...", q3l, fmt_mem(q3m), q4l, fmt_mem(q4m), kl, fmt_mem(km), fmt_mem(m)));

                    m
                };
                let t0 = std::time::Instant::now();

                *foresight += 1;

                {
                    let t0 = std::time::Instant::now();
                    let before = q3.len();
                    cq_par(threads, shards, &mut q3, |q3| {
                        q3.retain(|&(prev_idx, ref n)| {
                            let mut path = kns.materialize_cloned(prev_idx);
                            if let Some(kn) = n.key_node() {
                                path.push(kn.clone());
                            }
                            let mut path = Path::from_vec(path);
                            let n = n.clone();

                            deepen_search(ge, &mut path, n, *foresight - 1)
                        });
                    });
                    let after = q3.len();
                    le.log(LogLevel::INFO, format!("Deepened q3 {} => {}, foresight {} in {:?}", before, after, *foresight - 1, t0.elapsed()));

                    let t0 = std::time::Instant::now();
                    q3.defragment();
                    le.log(LogLevel::INFO, format!("Defragmented q3 in {:?}", t0.elapsed()));
                }

                {
                    let t0 = std::time::Instant::now();
                    let before = q4.len();
                    cq_par(threads, shards, &mut q4, |q4| {
                        q4.retain(|&(prev_idx, ref n)| {
                            let path = kns.materialize_cloned(prev_idx);
                            let mut path = Path::from_vec(path);
                            let n = n.clone();

                            deepen_search(ge, &mut path, n, *foresight - 1)
                        });
                    });
                    let after = q4.len();
                    le.log(LogLevel::INFO, format!("Deepened q4 {} => {}, foresight {} in {:?}", before, after, *foresight - 1, t0.elapsed()));

                    let t0 = std::time::Instant::now();
                    q4.defragment();
                    le.log(LogLevel::INFO, format!("Defragmented q4 in {:?}", t0.elapsed()));
                }

                let living = vec![].into_iter();
                let living = living.chain(q3.iter().map(|&(idx, _)| idx));
                let living = living.chain(q4.iter().map(|&(idx, _)| idx));
                let live_remap = kns.rebuild(living, |msg| le.log(LogLevel::INFO, msg));

                {
                    let t0 = std::time::Instant::now();
                    cq_par(threads, shards, &mut q3, |q3| {
                        for (idx, _) in q3.iter_mut() {
                            *idx = *live_remap.get(idx).unwrap();
                        }
                    });
                    le.log(LogLevel::INFO, format!("Reindexed q3 in {:?}", t0.elapsed()));
                }

                {
                    let t0 = std::time::Instant::now();
                    cq_par(threads, shards, &mut q4, |q4| {
                        for (idx, _) in q4.iter_mut() {
                            *idx = *live_remap.get(idx).unwrap();
                        }
                    });
                    le.log(LogLevel::INFO, format!("Reindexed q4 in {:?}", t0.elapsed()));
                }

                {
                    let q3m = q_mem(&q3);
                    let q4m = q_mem(&q4);
                    let km = kns_mem(&kns);
                    let m = q3m + q4m + km;

                    le.log(LogLevel::INFO, format!("Deepening pass {} => {} completed in {:?}", fmt_mem(m_before), fmt_mem(m), t0.elapsed()));
                }
            }
        }

        // start over
        *q = q4;
        *foresight = match *foresight {
            0 => 0,
            _ => *foresight - 1,
        };
        *depth += 1;

        le.log(LogLevel::INFO, format!("Completed BFS step to depth {}", *depth));

        if let Some(&(idx, ref n)) = q.front() {
            le.on_recollect_firstest((kns.materialize_cloned(idx), n.clone()));
        }
        if !le.on_recollect_results(r) {
            break;
        }
    }
}

fn kns_mem<N: Default>(kns: &KnPile<N>) -> usize {
    // whatever kns thinks plus (usize, usize) for space during recompaction
    kns.len() * (kns.esize() + std::mem::size_of::<(usize, usize)>())
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

    let k = (1 << 10);
    if mem >= k {
        return format!("{:.2} KB", (mem as f64) / (k as f64));
    }

    return format!("{} B", mem);
}

fn deepen_search<N: DfsNode, GE: DfsGraph<N>>(ge: &GE, path: &mut Path<N>, n: N, foresight: usize) -> bool {
    if foresight == 0 {
        return true;
    }

    for n2 in ge.expand(&n, path.kn_iter()) {
        let kn2 = n2.key_node();
        if let Some(kn2) = &kn2 {
            if let Some(_) = ge.end(kn2, path.kn_iter()) {
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

fn singleton_par<T: Send>(threads: usize, ts: &mut Vec<T>, f: impl Fn(&mut T) + Sync) {
    let wq = SegQueue::new();
    for t in ts.iter_mut() {
        wq.push(t);
    }

    crossbeam::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|_| {
                loop {
                    let t = match wq.pop() {
                        Ok(t) => t,
                        Err(PopError) => {
                            return;
                        }
                    };

                    f(t);
                }
            });
        }
    }).unwrap();
}

fn cq_par<T: Send>(threads: usize, shards: usize, ts: &mut ChunkQueue<T>, f: impl Fn(&mut ChunkQueue<T>) + Sync) {
    let mut tss = ts.drain_partition(shards);

    singleton_par(threads, &mut tss, f);

    for mut ts1 in tss {
        ts.append(&mut ts1);
    }
}
