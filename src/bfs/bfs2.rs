#![allow(unused_parens)]

use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
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
    q2: ChunkQueue<(usize, N, Option<N::KN>)>,
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

    fn mem(&self, kns: &KnPile<N::KN>) -> usize {
        let qm = q_mem(&self.q);
        // TODO: this is freaking awful, e.g.  when q was 40b per q2 was 88b per (!)
        let q2em = q_el_mem(&self.q2).max(q_el_mem(&self.q) + kns_el_mem(kns));
        let q2m = self.q2.len() * q2em;
        qm + q2m
    }
}

pub fn bfs2<N: DfsNode, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N> + Sync>(n0s: Vec<N>, ge: &GE, le: &mut LE) {
    let mem_max = (8 << 30);

    let threads = le.threads();
    let shards = threads * 10;

    let mut kns = KnPile::of(n0s.iter().map(|n0| n0.key_node().unwrap()));
    let mut q = ChunkQueue::new();
    for (idx, n0) in n0s.into_iter().enumerate() {
        q.push_back((idx, n0));
    }
    let mut foresight = 0;
    let mut depth = 0;

    loop {
        let size = q.len();

        if size == 0 {
            break;
        }

        {
            let ql = q.len();
            let qm = ql * q_el_mem(&q);
            let kl = kns.len();
            let km = kl * kns_el_mem(&kns);
            let m = qm + km;
            le.log(LogLevel::INFO, format!("Queue {} ({}), kns {} ({}), total {}", ql, fmt_mem(qm), kl, fmt_mem(km), fmt_mem(m)));
        }

        // step1: split q into work units
        let mut ws: Vec<_> = q.drain_partition(shards).into_iter().map(|q| WorkUnit::new(q)).collect();

        // step2: expand until all work units are done, deepening as needed
        loop {
            let stop = AtomicBool::new(false);

            // step 2a: expand as much as we can until/unless we go over memory
            {
                let mem = ws.iter().map(|w| w.mem(&kns)).sum::<usize>() + kns_mem(&kns);

                let wq = SegQueue::new();
                for w in ws.iter_mut() {
                    wq.push(w);
                }

                let mem = AtomicUsize::new(mem);

                crossbeam::scope(|sc| {
                    for _ in 0..threads {
                        sc.spawn(|_| {
                            loop {
                                let w = match wq.pop() {
                                    Ok(w) => w,
                                    Err(PopError) => {
                                        return;
                                    }
                                };

                                loop {
                                    if stop.load(Ordering::Relaxed) {
                                        return;
                                    }

                                    let wm0 = w.mem(&kns);
                                    let (prev_idx, n) = match w.q.pop_front() {
                                        Some(p) => p,
                                        None => {
                                            break;
                                        }
                                    };

                                    for n2 in ge.expand(&n) {
                                        let kn2 = n2.key_node();
                                        if let Some(kn2) = &kn2 {
                                            if let Some(label) = ge.end(kn2) {
                                                let mut path = kns.materialize_cloned(prev_idx);
                                                path.push(kn2.clone());
                                                le.debug_end(&path, label);
                                                w.r.add_end(path, label);
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
                                                w.r.add_cycle(path, cycle, kn2.clone());
                                                continue;
                                            }
                                        }

                                        w.q2.push_back((prev_idx, n2, kn2));
                                    }

                                    let wm1 = w.mem(&kns);

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
                            }
                        });
                    }
                }).unwrap();
            }

            if stop.load(Ordering::Relaxed) {
                // step2b: deepen (as needed)
                {
                    let ql = ws.iter().map(|w| w.q.len()).sum::<usize>();
                    let q2l = ws.iter().map(|w| w.q2.len()).sum::<usize>();
                    let wm = ws.iter().map(|w| w.mem(&kns)).sum::<usize>();
                    let kl = kns.len();
                    let km = kl * kns_el_mem(&kns);
                    let m = wm + km;
                    le.log(LogLevel::INFO, format!("Queue [{}, {}] ({}), kns {} ({}), total {}, deepening...", ql, q2l, fmt_mem(wm), kl, fmt_mem(km), fmt_mem(m)));
                }

                foresight += 1;

                let t0 = std::time::Instant::now();
                {
                    let wq = SegQueue::new();
                    for w in ws.iter_mut() {
                        wq.push(w);
                    }

                    crossbeam::scope(|sc| {
                        for _ in 0..threads {
                            sc.spawn(|_| {
                                loop {
                                    let w = match wq.pop() {
                                        Ok(w) => w,
                                        Err(PopError) => {
                                            return;
                                        }
                                    };

                                    w.q.retain(|&(prev_idx, ref n)| {
                                        let path = kns.materialize_cloned(prev_idx);
                                        let mut path = Path::from_vec(path);
                                        let n = n.clone();

                                        deepen_search(ge, &mut path, n, foresight)
                                    });
                                    w.q2.retain(|&(prev_idx, ref n, ref kn)| {
                                        let mut path = kns.materialize_cloned(prev_idx);
                                        if let Some(kn) = kn {
                                            path.push(kn.clone());
                                        }
                                        let mut path = Path::from_vec(path);
                                        let n = n.clone();

                                        deepen_search(ge, &mut path, n, foresight - 1)
                                    });
                                }
                            });
                        }
                    }).unwrap();
                }

                {
                    let ql = ws.iter().map(|w| w.q.len()).sum::<usize>();
                    let q2l = ws.iter().map(|w| w.q2.len()).sum::<usize>();
                    let wm = ws.iter().map(|w| w.mem(&kns)).sum::<usize>();
                    let kl = kns.len();
                    let km = kl * kns_el_mem(&kns);
                    let m = wm + km;
                    le.log(LogLevel::INFO, format!("Deepened to queue [{}, {}] ({}), kns {} ({}), total {}, foresight {} in {:?}", ql, q2l, fmt_mem(wm), kl, fmt_mem(km), fmt_mem(m), foresight, t0.elapsed()));
                }

                let living = vec![].into_iter();
                let living = living.chain(ws.iter().map(|w| w.q.iter().map(|&(idx, _)| idx)).flatten());
                let living = living.chain(ws.iter().map(|w| w.q2.iter().map(|&(idx, _, _)| idx)).flatten());
                let live_remap = kns.rebuild(living, |msg| le.log(LogLevel::INFO, msg));

                let t0 = std::time::Instant::now();
                {
                    let wq = SegQueue::new();
                    for w in ws.iter_mut() {
                        wq.push(w);
                    }

                    crossbeam::scope(|sc| {
                        for _ in 0..threads {
                            sc.spawn(|_| {
                                loop {
                                    let w = match wq.pop() {
                                        Ok(w) => w,
                                        Err(PopError) => {
                                            return;
                                        }
                                    };

                                    for (idx, _) in w.q.iter_mut() {
                                        *idx = *live_remap.get(idx).unwrap();
                                    }
                                    for (idx, _, _) in w.q2.iter_mut() {
                                        *idx = *live_remap.get(idx).unwrap();
                                    }
                                }
                            });
                        }
                    }).unwrap();
                }
                le.log(LogLevel::INFO, format!("Reindexed in {:?}", t0.elapsed()));
            }
            else {
                // finished, great
                break;
            }
        }

        // step3: fold work unit results together/into kns
        let mut q2 = ChunkQueue::new();
        let mut r = DfsRes::new();
        for mut w in ws {
            assert_eq!(0, w.q.len());

            for (prev_idx, n, kn) in w.q2.into_iter() {
                let mut prev_idx = prev_idx;
                if let Some(kn) = kn {
                    prev_idx = kns.push(prev_idx, kn);
                }
                q2.push_back((prev_idx, n));
            }

            r.append(&mut w.r);
        }

        // start over
        q = q2;
        foresight = match foresight {
            0 => 0,
            _ => foresight - 1,
        };
        depth += 1;

        le.log(LogLevel::INFO, format!("Completed BFS step to depth {}", depth));

        if let Some(&(idx, ref n)) = q.front() {
            le.on_recollect_firstest((kns.materialize_cloned(idx), n.clone()));
        }
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

fn q_el_mem<T>(_q: &ChunkQueue<T>) -> usize {
    std::mem::size_of::<T>()
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

fn deepen<T: Send, N: DfsNode, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N>, F: Fn(&T) -> (Path<N>, N) + Send + Sync>(ge: &GE, le: &mut LE, threads: usize, name: &'static str, q: &mut ChunkQueue<T>, foresight: usize, f: F) {
    if foresight == 0 {
        return;
    }
    let size0 = q.len();
    if size0 == 0 {
        return;
    }

    let t0 = std::time::Instant::now();
    le.log(LogLevel::INFO, format!("Deepening {} from size {}...", name, size0));

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

    le.log(LogLevel::INFO, format!("Deepened {} from size {} to size {} foresight {} in {:?}", name, size0, q.len(), foresight, t0.elapsed()));
}

fn deepen_search<N: DfsNode, GE: DfsGraph<N>>(ge: &GE, path: &mut Path<N>, n: N, foresight: usize) -> bool {
    if foresight == 0 {
        return true;
    }

    for n2 in ge.expand(&n) {
        let kn2 = n2.key_node();
        if let Some(kn2) = &kn2 {
            if let Some(_) = ge.end(kn2) {
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
