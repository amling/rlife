use crate::chunk_store;

use chunk_store::ChunkFactory;
use chunk_store::ChunksVec;

pub struct KnPile<N: Default, CF: ChunkFactory<(usize, N, usize)>> {
    inner: ChunksVec<(usize, N, usize), CF>,
}

pub trait KnsRebuildable {
    fn walk(&mut self, f: impl FnMut(&mut usize));
}

impl<N: Default, CF: ChunkFactory<(usize, N, usize)>> KnPile<N, CF> {
    pub fn new(cf: CF) -> Self {
        let mut inner = ChunksVec::new(cf);
        inner.push((0, N::default(), 0));
        KnPile {
            inner: inner,
        }
    }

    pub fn rebuild(&mut self, mut pins: impl KnsRebuildable, log: impl FnOnce(String)) {
        let t0 = std::time::Instant::now();
        let size = self.len();

        // mark living
        pins.walk(|&mut idx| self.inner[idx].2 = 1);

        // walk right-to-left propagating living
        let len = self.len();
        for idx in (1..len).rev() {
            let (prev_idx, _, live) = self.inner[idx];
            if live == 0 {
                // we're not gonna live ourselves so we don't retain parent
                continue;
            }
            if prev_idx == 0 {
                // we're a first-level, don't worry about zero root
                continue;
            }
            debug_assert!(prev_idx < idx);
            self.inner[prev_idx].2 = 1;
        }

        // walk left-to-right deciding new positions
        let mut new_idx = 1;
        for idx in 1..len {
            let (_, _, live) = &mut self.inner[idx];
            if *live == 0 {
                // we're not gonna live, we don't get a number
                continue;
            }
            *live = new_idx;
            new_idx += 1;
        }

        // reindex ourselves
        for idx in 1..len {
            let (prev_idx, _, new_idx) = self.inner[idx];
            if new_idx == 0 {
                // we're not gonna live, don't worry about reindexing ourselves
                continue;
            }
            if prev_idx == 0 {
                // our parent is zero root, no need to reindex
                continue;
            }
            let (_, _, new_prev_idx) = self.inner[prev_idx];
            debug_assert!(new_prev_idx != 0);
            self.inner[idx].0 = new_prev_idx;
        }

        // reindex our callers
        pins.walk(|idx| {
            if *idx == 0 {
                return;
            }
            let (_, _, new_idx) = self.inner[*idx];
            debug_assert!(new_idx != 0);
            *idx = new_idx;
        });

        // compact left-to-right
        let (rebuilt_len, root_ct) = {
            let mut i = 1;
            let mut j = 1;
            let mut root_ct = 0;
            loop {
                if j >= len {
                    break;
                }

                let &mut (prev_idx, ref mut n, new_idx) = &mut self.inner[j];
                if new_idx == 0 {
                    j += 1;
                    continue;
                }

                if prev_idx == 0 {
                    root_ct += 1;
                }

                debug_assert!(i == new_idx);
                self.inner[i] = (prev_idx, std::mem::take(n), 0);

                i += 1;
                j += 1;
            }
            (i, root_ct)
        };

        self.inner.truncate(rebuilt_len);
        debug_assert_eq!(self.len(), rebuilt_len);

        log(format!("Rebuilt kns from {} to {} ({} roots) in {:?}", size, rebuilt_len, root_ct, t0.elapsed()));
    }

    pub fn materialize<T>(&self, idx: usize, mut f: impl FnMut(&N) -> T) -> Vec<T> {
        let mut acc = Vec::new();
        let mut idx = idx;
        while idx != 0 {
            let (prev_idx, ref n, _) = self.inner[idx];
            acc.push(f(n));
            idx = prev_idx;
        }
        acc.reverse();
        acc
    }

    pub fn find<T>(&self, idx: usize, mut f: impl FnMut(usize, usize, &N) -> Option<T>) -> Option<T> {
        let mut idx = idx;
        while idx != 0 {
            let (prev_idx, ref n, _) = self.inner[idx];
            if let Some(t) = f(idx, prev_idx, n) {
                return Some(t);
            }
            idx = prev_idx;
        }
        None
    }

    pub fn push(&mut self, idx: usize, n: N) -> usize {
        self.inner.push((idx, n, 0))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn esize(&self) -> usize {
        self.inner.esize()
    }

    pub fn materialize_cloned(&self, idx: usize) -> Vec<N> where N: Clone {
        self.materialize(idx, |n| n.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item=(usize, &N)> {
        self.inner.iter().map(|&(idx, ref n, _)| (idx, n))
    }
}
