#![allow(unused_parens)]

use ars_ds::err::StringError;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::io::Read;
use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::bfs;
use crate::chunk_store;
use crate::dfs;
use crate::sal;

use bfs::chunk_queue::ChunkQueue;
use bfs::kn_pile::KnPile;
use bfs::kn_pile::KnsRebuildable;
use chunk_store::ChunkFactory;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use dfs::res::DfsRes;
use sal::DeserializerFor;
use sal::SerializerFor;

pub trait Bfs2ChunkFactory<N: DfsNode>: ChunkFactory<(usize, N)> + ChunkFactory<(usize, N::KN, usize)> {
}

impl<N: DfsNode, T: ChunkFactory<(usize, N)> + ChunkFactory<(usize, N::KN, usize)>> Bfs2ChunkFactory<N> for T {
}

struct WorkUnit<N: DfsNode, CF: Bfs2ChunkFactory<N>> {
    q: ChunkQueue<(usize, N), CF>,
    q2: ChunkQueue<(usize, N), CF>,
    r: DfsRes<N::KN>,
}

impl<N: DfsNode, CF: Bfs2ChunkFactory<N>> WorkUnit<N, CF> {
    fn new(q: ChunkQueue<(usize, N), CF>) -> Self {
        let cf = q.cf;
        WorkUnit {
            q: q,
            q2: ChunkQueue::new(cf),
            r: DfsRes::new(),
        }
    }
}

pub trait Bfs2Dedupe<N: DfsNode> {
    fn new() -> Self;
    fn len(&self) -> usize;
    fn cloned_iter<'a>(&'a self) -> Box<dyn Iterator<Item=<N::KN as DfsKeyNode>::HN> + 'a>;
    fn insert(&mut self, n: <N::KN as DfsKeyNode>::HN) -> bool;
}

impl<N: DfsNode> Bfs2Dedupe<N> for HashSet<<N::KN as DfsKeyNode>::HN> {
    fn new() -> Self {
        HashSet::new()
    }

    fn len(&self) -> usize {
        HashSet::len(self)
    }

    fn cloned_iter<'a>(&'a self) -> Box<dyn Iterator<Item=<N::KN as DfsKeyNode>::HN> + 'a> {
        Box::new(HashSet::iter(self).cloned())
    }

    fn insert(&mut self, n: <N::KN as DfsKeyNode>::HN) -> bool {
        HashSet::insert(self, n)
    }
}

pub struct Bfs2State<N: DfsNode, CF: Bfs2ChunkFactory<N>, D: Bfs2Dedupe<N>> {
    kns: KnPile<N::KN, CF>,
    q: ChunkQueue<(usize, N), CF>,
    dedupe: D,
    foresight: usize,
    depth: usize,
}

pub struct Bfs2CustomSerializer<CF>(pub CF);

impl<N: DfsNode, CF: Bfs2ChunkFactory<N>, D: Bfs2Dedupe<N>> SerializerFor<Bfs2State<N, CF, D>> for Bfs2CustomSerializer<CF> where N: Serialize, N::KN: Serialize, <N::KN as DfsKeyNode>::HN: Serialize {
    fn to_writer(&self, mut w: impl Write, s: &Bfs2State<N, CF, D>) -> Result<(), StringError> {
        w.write_u64::<BigEndian>(s.kns.len() as u64)?;
        for e in s.kns.iter().skip(1) {
            let e: (usize, &N::KN) = e;
            bincode::serialize_into(w.by_ref(), &e)?;
        }

        w.write_u64::<BigEndian>(s.q.len() as u64)?;
        for e in s.q.iter() {
            let e: &(usize, N) = e;
            bincode::serialize_into(w.by_ref(), e)?;
        }

        w.write_u64::<BigEndian>(s.dedupe.len() as u64)?;
        for e in s.dedupe.cloned_iter() {
            let e: <N::KN as DfsKeyNode>::HN = e;
            bincode::serialize_into(w.by_ref(), &e)?;
        }

        w.write_u64::<BigEndian>(s.foresight as u64)?;
        w.write_u64::<BigEndian>(s.depth as u64)?;
        Ok(())
    }
}

impl<N: DfsNode, CF: Bfs2ChunkFactory<N>, D: Bfs2Dedupe<N>> DeserializerFor<Bfs2State<N, CF, D>> for Bfs2CustomSerializer<CF> where N: DeserializeOwned + Copy, N::KN: DeserializeOwned, <N::KN as DfsKeyNode>::HN: DeserializeOwned {
    fn from_reader(&self, mut r: impl Read) -> Result<Bfs2State<N, CF, D>, StringError> {
        let kns = {
            let len = r.read_u64::<BigEndian>()? as usize;
            let mut kns = KnPile::new(self.0);
            for _ in 1..len {
                let e: (usize, N::KN) = bincode::deserialize_from(r.by_ref())?;
                kns.push(e.0, e.1);
            }
            kns
        };

        let q = {
            let len = r.read_u64::<BigEndian>()? as usize;
            let mut q = ChunkQueue::new(self.0);
            for _ in 0..len {
                let e: (usize, N) = bincode::deserialize_from(r.by_ref())?;
                q.push_back(e);
            }
            q
        };

        let dedupe = {
            let len = r.read_u64::<BigEndian>()? as usize;
            let mut dedupe = D::new();
            for _ in 0..len {
                let e: <N::KN as DfsKeyNode>::HN = bincode::deserialize_from(r.by_ref())?;
                dedupe.insert(e);
            }
            dedupe
        };

        let foresight = r.read_u64::<BigEndian>()? as usize;
        let depth = r.read_u64::<BigEndian>()? as usize;
        Ok(Bfs2State {
            kns: kns,
            q: q,
            dedupe: dedupe,
            foresight: foresight,
            depth: depth,
        })
    }
}

impl<N: DfsNode, CF: Bfs2ChunkFactory<N>, D: Bfs2Dedupe<N>> Bfs2State<N, CF, D> {
    pub fn new_simple(n0: N, cf: CF) -> Bfs2State<N, CF, D> where N: Copy {
        Self::new(vec![(vec![n0.key_node().unwrap()], n0)], cf)
    }

    pub fn new(init: impl IntoIterator<Item=(Vec<N::KN>, N)>, cf: CF) -> Bfs2State<N, CF, D> where N: Copy {
        let mut kns = KnPile::new(cf);
        let mut q = ChunkQueue::new(cf);

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
            // Dubious that we miss the initial states?  Unfortunately there's no way to get access
            // to dedupe predicate here.
            dedupe: D::new(),
            foresight: 0,
            depth: 0,
        }
    }

    pub fn serializer(&self) -> impl SerializerFor<Bfs2State<N, CF, D>> where N: Serialize, N::KN: Serialize, <N::KN as DfsKeyNode>::HN: Serialize {
        Bfs2CustomSerializer(self.q.cf)
    }
}

impl<'a, N, CF: ChunkFactory<(usize, N)>> KnsRebuildable for &'a mut ChunkQueue<(usize, N), CF> {
    fn walk(&mut self, mut f: impl FnMut(&mut usize)) {
        for (idx, _) in self.iter_mut() {
            f(idx);
        }
    }
}

impl<'a, N: DfsNode, CF: Bfs2ChunkFactory<N>> KnsRebuildable for &'a mut Vec<WorkUnit<N, CF>> {
    fn walk(&mut self, mut f: impl FnMut(&mut usize)) {
        for w in self.iter_mut() {
            for (idx, _) in w.q.iter_mut() {
                f(idx);
            }
            for (idx, _) in w.q2.iter_mut() {
                f(idx);
            }
        }
    }
}

impl<A: KnsRebuildable, B: KnsRebuildable> KnsRebuildable for (A, B) {
    fn walk(&mut self, mut f: impl FnMut(&mut usize)) {
        self.0.walk(&mut f);
        self.1.walk(&mut f);
    }
}

pub fn bfs2<N: DfsNode + Copy, CF: Bfs2ChunkFactory<N>, D: Bfs2Dedupe<N>, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N> + Sync>(state: Bfs2State<N, CF, D>, ge: &GE, le: &mut LE) {
    bfs2_dedupe(state, ge, le, |_| false)
}

pub fn bfs2_dedupe<N: DfsNode + Copy, CF: Bfs2ChunkFactory<N>, D: Bfs2Dedupe<N>, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N> + Sync>(mut state: Bfs2State<N, CF, D>, ge: &GE, le: &mut LE, should_dedupe: impl Fn(&<N::KN as DfsKeyNode>::HN) -> bool) {
    loop {
        let cf = state.q.cf;

        let threads = le.threads();
        let shards = threads * 100;

        le.debug_bfs2_checkpoint(|le| {
            let kns = &mut state.kns;
            let q = &mut state.q;

            kns.rebuild(q, |msg| le.log(LogLevel::INFO, msg));

            &state
        });

        let kns = &mut state.kns;
        let q = &mut state.q;
        let dedupe = &mut state.dedupe;
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

                    for n2 in ge.expand(&n) {
                        let kn2 = n2.key_node();
                        if let Some(kn2) = &kn2 {
                            if let Some(label) = ge.end(kn2) {
                                let mut path = kns.materialize_cloned(prev_idx);
                                path.push(kn2.clone());
                                le.debug_end(&path, label);
                                w.r.add_end(path, label);
                                // continue;
                            }

                            let hn2 = kn2.hash_node();
                            let find_f = |idx, _prev_idx, kn: &N::KN| {
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
                        w.q.retain(|(_prev_idx, n)| {
                            deepen_search(ge, n, *foresight)
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
                        w.q2.retain(|(_prev_idx, n)| {
                            deepen_search(ge, n, *foresight - 1)
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

                kns.rebuild(&mut ws, |msg| le.log(LogLevel::INFO, msg));

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
        let mut q3 = ChunkQueue::new(cf);
        let mut r = DfsRes::new();
        for mut w in ws {
            assert_eq!(0, w.q.len());

            q3.append(&mut w.q2);
            r.append(&mut w.r);
        }

        // step3b: fold results into kns
        let mut mem_max = le.max_mem();
        let mut q4 = ChunkQueue::new(cf);
        while let Some((prev_idx, n)) = q3.pop_front() {
            let mut prev_idx = prev_idx;
            if let Some(kn) = n.key_node() {
                if let Some(hn) = kn.hash_node() {
                    if should_dedupe(&hn) {
                        if !dedupe.insert(hn) {
                            continue;
                        }
                    }
                }
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
                        q3.retain(|(_prev_idx, n)| {
                            deepen_search(ge, n, *foresight - 1)
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
                        q4.retain(|(_prev_idx, n)| {
                            deepen_search(ge, n, *foresight - 1)
                        });
                    });
                    let after = q4.len();
                    le.log(LogLevel::INFO, format!("Deepened q4 {} => {}, foresight {} in {:?}", before, after, *foresight - 1, t0.elapsed()));

                    let t0 = std::time::Instant::now();
                    q4.defragment();
                    le.log(LogLevel::INFO, format!("Defragmented q4 in {:?}", t0.elapsed()));
                }

                kns.rebuild((&mut q3, &mut q4), |msg| le.log(LogLevel::INFO, msg));

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
        if dedupe.len() > 0 {
            le.log(LogLevel::INFO, format!("Dedupe size: {}", dedupe.len()));
        }

        if let Some(&(idx, ref n)) = q.front() {
            le.on_recollect_firstest((kns.materialize_cloned(idx), n.clone()));
        }
        if !le.on_recollect_results(r) {
            break;
        }
    }
}

fn kns_mem<N: Default, CF: ChunkFactory<(usize, N, usize)>>(kns: &KnPile<N, CF>) -> usize {
    kns.len() * kns.esize()
}

fn q_mem<T, CF: ChunkFactory<T>>(q: &ChunkQueue<T, CF>) -> usize {
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

fn deepen_search<N: DfsNode, GE: DfsGraph<N>>(ge: &GE, n: &N, foresight: usize) -> bool {
    if foresight == 0 {
        return true;
    }

    for n2 in ge.expand(n) {
        if let Some(kn2) = n2.key_node() {
            if let Some(_) = ge.end(&kn2) {
                return true;
            }
        }

        if deepen_search(ge, &n2, foresight - 1) {
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

fn cq_par<T: Send, CF: ChunkFactory<T>>(threads: usize, shards: usize, ts: &mut ChunkQueue<T, CF>, f: impl Fn(&mut ChunkQueue<T, CF>) + Sync) {
    let mut tss = ts.drain_partition(shards);

    singleton_par(threads, &mut tss, f);

    for mut ts1 in tss {
        ts.append(&mut ts1);
    }
}
