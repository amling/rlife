use std::collections::HashMap;

pub struct KnPile<N> {
    pile: Vec<Vec<(usize, N)>>,
}

impl<N> KnPile<N> {
    fn shard_size(&self) -> usize {
        let d = self.esize();
        ((1 << 20) + d) / d
    }

    fn split_index(&self, idx: usize) -> (usize, usize) {
        let shard_size = self.shard_size();
        (idx / shard_size, idx % shard_size)
    }

    fn join_index(&self, outer: usize, inner: usize) -> usize {
        outer * self.shard_size() + inner
    }

    pub fn new() -> Self {
        KnPile {
            pile: Vec::new(),
        }
    }

    pub fn of(n0s: impl IntoIterator<Item=N>) -> Self {
        let mut r = Self::new();
        for (idx, n0) in n0s.into_iter().enumerate() {
            assert_eq!(idx, r.push(idx, n0));
        }
        r
    }

    fn get(&self, idx: usize) -> &(usize, N) {
        let (outer, inner) = self.split_index(idx);
        &self.pile[outer][inner]
    }

    fn get_mut(&mut self, idx: usize) -> &mut (usize, N) {
        let (outer, inner) = self.split_index(idx);
        &mut self.pile[outer][inner]
    }

    fn swap(&mut self, idxa: usize, idxb: usize) {
        let idx1 = idxa.min(idxb);
        let idx2 = idxa.max(idxb);

        let (outer1, inner1) = self.split_index(idx1);
        let (outer2, inner2) = self.split_index(idx2);

        if outer1 == outer2 {
            self.pile[outer1].swap(inner1, inner2);
        }
        else {
            let (s1, s2) = self.pile.split_at_mut(outer2);
            let r1 = &mut s1[outer1][inner1];
            let r2 = &mut s2[0][inner2];
            std::mem::swap(r1, r2);
        }
    }

    pub fn rebuild(&mut self, live: impl Iterator<Item=usize>) -> HashMap<usize, usize> {
        let t0 = std::time::Instant::now();
        let size = self.len();

        let mut live: Vec<_> = live.collect();
        live.sort();
        live.dedup();
        live.reverse();
        let mut i = 0;
        while i < live.len() {
            let idx = live[i];
            if idx != 0 {
                let prev_idx = self.get(idx).0;
                let last = *live.last().unwrap();
                if prev_idx == idx {
                    // root, fine
                }
                else if prev_idx < last {
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
        let mut root_ct = 0;
        while let Some(idx) = live.pop() {
            assert!(rebuilt_idx <= idx, "{} <= {}?", rebuilt_idx, idx);
            self.swap(rebuilt_idx, idx);
            // insert ourselves first so self-parent of roots can be looked up
            assert!(!live_remap.contains_key(&idx));
            live_remap.insert(idx, rebuilt_idx);
            let parent_idx = self.get(rebuilt_idx).0;
            let parent_idx = *live_remap.get(&parent_idx).unwrap();
            if parent_idx == rebuilt_idx {
                root_ct += 1;
            }
            self.get_mut(rebuilt_idx).0 = parent_idx;
            rebuilt_idx += 1;
        }

        let (rebuilt_outer, rebuilt_inner) = self.split_index(rebuilt_idx);
        self.pile.truncate(rebuilt_outer + 1);
        self.pile[rebuilt_outer].truncate(rebuilt_inner);
        debug_assert_eq!(self.len(), rebuilt_idx);

        eprintln!("Rebuilt kns from {} to {} ({} roots) in {:?}", size, rebuilt_idx, root_ct, t0.elapsed());

        live_remap
    }

    pub fn materialize<T>(&self, idx: usize, mut f: impl FnMut(&N) -> T) -> Vec<T> {
        let mut r = Vec::new();
        let mut idx = idx;
        loop {
            r.push(f(&self.get(idx).1));

            let next_idx = self.get(idx).0;

            if idx == next_idx {
                break;
            }

            idx = next_idx;
        }
        r.reverse();
        r
    }

    pub fn find<T>(&self, idx: usize, mut f: impl FnMut(usize, &N) -> Option<T>) -> Option<T> {
        let mut idx = idx;
        loop {
            if let Some(t) = f(idx, &self.get(idx).1) {
                return Some(t);
            }

            let next_idx = self.get(idx).0;

            if idx == next_idx {
                return None;
            }

            idx = next_idx;
        }
    }

    pub fn push(&mut self, idx: usize, n: N) -> usize {
        let shard_size = self.shard_size();
        let len = self.pile.len();
        if let Some(last) = self.pile.last_mut() {
            if last.len() < shard_size {
                let outer = len - 1;
                let inner = last.len();
                last.push((idx, n));
                return self.join_index(outer, inner);
            }
        }
        self.pile.push(Vec::with_capacity(shard_size));
        self.pile.last_mut().unwrap().push((idx, n));
        self.join_index(len, 0)
    }

    pub fn len(&self) -> usize {
        self.pile.iter().map(|v| v.len()).sum()
    }

    pub fn esize(&self) -> usize {
        std::mem::size_of::<(usize, N)>()
    }
}

impl<N: Clone> KnPile<N> {
    pub fn materialize_cloned(&self, idx: usize) -> Vec<N> {
        self.materialize(idx, |n| n.clone())
    }
}
