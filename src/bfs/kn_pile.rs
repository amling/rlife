use std::collections::HashMap;

pub struct KnPile<N> {
    pile: Vec<(usize, N)>,
}

impl<N> KnPile<N> {
    pub fn new(n0: N) -> Self {
        KnPile {
            pile: vec![(0, n0)],
        }
    }

    pub fn rebuild(&mut self, live: impl Iterator<Item=usize>) -> HashMap<usize, usize> {
        let mut live: Vec<_> = live.collect();
        live.sort();
        live.dedup();
        live.reverse();
        let mut i = 0;
        while i < live.len() {
            let idx = live[i];
            if idx != 0 {
                let prev_idx = self.pile[idx].0;
                let last = *live.last().unwrap();
                if prev_idx < last {
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
        while let Some(idx) = live.pop() {
            assert!(rebuilt_idx <= idx, "{} <= {}?", rebuilt_idx, idx);
            self.pile.swap(rebuilt_idx, idx);
            // insert ourselves first so link from 0 to 0 can be looked up
            assert_eq!(idx == 0, rebuilt_idx == 0);
            assert!(!live_remap.contains_key(&idx));
            live_remap.insert(idx, rebuilt_idx);
            self.pile[rebuilt_idx].0 = *live_remap.get(&self.pile[rebuilt_idx].0).unwrap();
            rebuilt_idx += 1;
        }

        self.pile.truncate(rebuilt_idx);
        live_remap
    }

    pub fn materialize<T>(&self, idx: usize, mut f: impl FnMut(&N) -> T) -> Vec<T> {
        let mut r = Vec::new();
        let mut idx = idx;
        loop {
            r.push(f(&self.pile[idx].1));

            if idx == 0 {
                break;
            }

            idx = self.pile[idx].0;
        }
        r.reverse();
        r
    }

    pub fn find<T>(&self, idx: usize, mut f: impl FnMut(usize, &N) -> Option<T>) -> Option<T> {
        let mut idx = idx;
        loop {
            if let Some(t) = f(idx, &self.pile[idx].1) {
                return Some(t);
            }

            if idx == 0 {
                return None;
            }

            idx = self.pile[idx].0;
        }
    }

    pub fn push(&mut self, idx: usize, n: N) -> usize {
        let new_idx = self.pile.len();
        self.pile.push((idx, n));
        new_idx
    }

    pub fn len(&self) -> usize {
        self.pile.len()
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
