use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct KnPile<N: Default> {
    pile: Vec<Vec<(usize, N)>>,
}

impl<N: Default> KnPile<N> {
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
            pile: vec![vec![(0, N::default())]],
        }
    }

    fn get(&self, idx: usize) -> &(usize, N) {
        let (outer, inner) = self.split_index(idx);
        &self.pile[outer][inner]
    }

    fn get_mut(&mut self, idx: usize) -> &mut (usize, N) {
        debug_assert!(idx != 0);

        let (outer, inner) = self.split_index(idx);
        &mut self.pile[outer][inner]
    }

    fn swap(&mut self, idxa: usize, idxb: usize) {
        debug_assert!(idxa != 0);
        debug_assert!(idxb != 0);

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

    pub fn rebuild(&mut self, live: impl Iterator<Item=usize>, log: impl FnOnce(String)) -> HashMap<usize, usize> {
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
                if prev_idx == 0 {
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
        live_remap.insert(0, 0);
        let mut rebuilt_idx = 1;
        let mut root_ct = 0;
        while let Some(idx) = live.pop() {
            assert!(rebuilt_idx <= idx, "{} <= {}?", rebuilt_idx, idx);
            self.swap(rebuilt_idx, idx);

            let parent_idx = self.get(rebuilt_idx).0;
            let parent_idx = *live_remap.get(&parent_idx).unwrap();
            if parent_idx == 0 {
                root_ct += 1;
            }
            self.get_mut(rebuilt_idx).0 = parent_idx;

            assert!(!live_remap.contains_key(&idx));
            live_remap.insert(idx, rebuilt_idx);

            rebuilt_idx += 1;
        }

        {
            let (outer, inner) = self.split_index(rebuilt_idx - 1);
            self.pile.truncate(outer + 1);
            self.pile[outer].truncate(inner + 1);
        }

        debug_assert_eq!(self.len(), rebuilt_idx);

        log(format!("Rebuilt kns from {} to {} ({} roots) in {:?}", size, rebuilt_idx, root_ct, t0.elapsed()));

        live_remap
    }

    pub fn materialize<T>(&self, idx: usize, mut f: impl FnMut(&N) -> T) -> Vec<T> {
        let mut acc = Vec::new();
        let mut idx = idx;
        while idx != 0 {
            let r = self.get(idx);
            acc.push(f(&r.1));
            idx = r.0;
        }
        acc.reverse();
        acc
    }

    pub fn find<T>(&self, idx: usize, mut f: impl FnMut(usize, usize, &N) -> Option<T>) -> Option<T> {
        let mut idx = idx;
        while idx != 0 {
            let r = self.get(idx);
            if let Some(t) = f(idx, r.0, &r.1) {
                return Some(t);
            }
            idx = r.0;
        }
        None
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
        let len = match self.pile.last() {
            Some(last) => (self.pile.len() - 1) * self.shard_size() + last.len(),
            None => 0,
        };
        debug_assert_eq!(len, self.pile.iter().map(|v| v.len()).sum::<usize>());
        len
    }

    pub fn esize(&self) -> usize {
        std::mem::size_of::<(usize, N)>()
    }

    pub fn materialize_cloned(&self, idx: usize) -> Vec<N> where N: Clone {
        self.materialize(idx, |n| n.clone())
    }
}
