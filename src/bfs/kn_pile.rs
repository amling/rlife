use crate::chunk_store;

use chunk_store::ChunkFactory;
use chunk_store::ChunkVec;

pub struct KnPile<N: Default, CF: ChunkFactory<(usize, N, usize)>> {
    cf: CF,
    pile: Vec<ChunkVec<(usize, N, usize), CF::Output>>,
}

pub trait KnsRebuildable {
    fn walk(&mut self, f: impl FnMut(&mut usize));
}

fn esize<N>() -> usize {
    std::mem::size_of::<(usize, N, usize)>()
}

fn shard_size<N>() -> usize {
    let d = esize::<N>();
    (1 << 20) / d
}

impl<N: Default, CF: ChunkFactory<(usize, N, usize)>> KnPile<N, CF> {
    fn shard_size(&self) -> usize {
        shard_size::<N>()
    }

    fn split_index(&self, idx: usize) -> (usize, usize) {
        let shard_size = self.shard_size();
        (idx / shard_size, idx % shard_size)
    }

    fn join_index(&self, outer: usize, inner: usize) -> usize {
        outer * self.shard_size() + inner
    }

    pub fn new(cf: CF) -> Self {
        let mut v = cf.new_chunk_vec(shard_size::<N>());
        assert!(v.offer((0, N::default(), 0)));
        KnPile {
            cf: cf,
            pile: vec![v],
        }
    }

    fn get(&self, idx: usize) -> &(usize, N, usize) {
        let (outer, inner) = self.split_index(idx);
        &self.pile[outer][inner]
    }

    fn get_mut(&mut self, idx: usize) -> &mut (usize, N, usize) {
        debug_assert!(idx != 0);

        let (outer, inner) = self.split_index(idx);
        &mut self.pile[outer][inner]
    }

    pub fn rebuild(&mut self, mut pins: impl KnsRebuildable, log: impl FnOnce(String)) {
        let t0 = std::time::Instant::now();
        let size = self.len();

        // mark living
        pins.walk(|&mut idx| self.get_mut(idx).2 = 1);

        // walk right-to-left propagating living
        let len = self.len();
        for idx in (1..len).rev() {
            let &(prev_idx, _, live) = self.get(idx);
            if live == 0 {
                // we're not gonna live ourselves so we don't retain parent
                continue;
            }
            if prev_idx == 0 {
                // we're a first-level, don't worry about zero root
                continue;
            }
            debug_assert!(prev_idx < idx);
            self.get_mut(prev_idx).2 = 1;
        }

        // walk left-to-right deciding new positions
        let mut new_idx = 1;
        for idx in 1..len {
            let (_, _, live) = self.get_mut(idx);
            if *live == 0 {
                // we're not gonna live, we don't get a number
                continue;
            }
            *live = new_idx;
            new_idx += 1;
        }

        // reindex ourselves
        for idx in 1..len {
            let (prev_idx, _, new_idx) = self.get(idx);
            if *new_idx == 0 {
                // we're not gonna live, don't worry about reindexing ourselves
                continue;
            }
            if *prev_idx == 0 {
                // our parent is zero root, no need to reindex
                continue;
            }
            let &(_, _, new_prev_idx) = self.get(*prev_idx);
            debug_assert!(new_prev_idx != 0);
            self.get_mut(idx).0 = new_prev_idx;
        }

        // reindex our callers
        pins.walk(|idx| {
            if *idx == 0 {
                return;
            }
            let &(_, _, new_idx) = self.get(*idx);
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

                let &mut (prev_idx, ref mut n, new_idx) = self.get_mut(j);
                if new_idx == 0 {
                    j += 1;
                    continue;
                }

                if prev_idx == 0 {
                    root_ct += 1;
                }

                debug_assert!(i == new_idx);
                *self.get_mut(i) = (prev_idx, std::mem::take(n), 0);

                i += 1;
                j += 1;
            }
            (i, root_ct)
        };

        // truncate
        {
            let (outer, inner) = self.split_index(rebuilt_len - 1);
            self.pile.truncate(outer + 1);
            self.pile[outer].truncate(inner + 1);
        }

        debug_assert_eq!(self.len(), rebuilt_len);

        log(format!("Rebuilt kns from {} to {} ({} roots) in {:?}", size, rebuilt_len, root_ct, t0.elapsed()));
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
                assert!(last.offer((idx, n, 0)));
                return self.join_index(outer, inner);
            }
        }
        self.pile.push(self.cf.new_chunk_vec(shard_size));
        assert!(self.pile.last_mut().unwrap().offer((idx, n, 0)));
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
        esize::<N>()
    }

    pub fn materialize_cloned(&self, idx: usize) -> Vec<N> where N: Clone {
        self.materialize(idx, |n| n.clone())
    }

    pub fn iter(&self) -> impl Iterator<Item=(usize, &N)> {
        self.pile.iter().map(|c| c.iter().map(|&(idx, ref n, _)| (idx, n))).flatten()
    }
}
