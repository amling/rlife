#![allow(unused_parens)]

use std::collections::VecDeque;

pub struct ChunkQueue<N> {
    q: VecDeque<VecDeque<N>>,
}

impl<N> ChunkQueue<N> {
    fn chunk_size(&self) -> usize {
        let d = std::mem::size_of::<N>();
        ((1 << 20) + d) / d
    }

    pub fn new() -> Self {
        ChunkQueue {
            q: VecDeque::new(),
        }
    }

    pub fn push_back(&mut self, n: N) {
        let chunk_size = self.chunk_size();
        if let Some(last) = self.q.back_mut() {
            if last.len() < chunk_size {
                last.push_back(n);
                return;
            }
        }
        self.q.push_back(VecDeque::with_capacity(chunk_size));
        self.q.back_mut().unwrap().push_back(n);
    }

    pub fn len(&self) -> usize {
        self.q.iter().map(|q| q.len()).sum()
    }

    pub fn pop_front(&mut self) -> Option<N> {
        loop {
            match self.q.front_mut() {
                Some(q) => {
                    match q.pop_front() {
                        Some(n) => {
                            return Some(n);
                        }
                        None => {
                            self.q.pop_front().unwrap();
                            continue;
                        }
                    }
                }
                None => {
                    return None;
                }
            }
        }
    }

    pub fn front(&self) -> Option<&N> {
        for q in self.q.iter() {
            if let Some(n) = q.front() {
                return Some(n);
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item=&N> {
        self.q.iter().map(|q| q.iter()).flatten()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut N> {
        self.q.iter_mut().map(|q| q.iter_mut()).flatten()
    }

    pub fn drain_partition(&mut self, shards: usize) -> Vec<ChunkQueue<N>> {
        let len = self.q.len();
        let ret = (0..shards).map(|i| {
            let ct = ((i + 1) * len / shards - i * len / shards);
            ChunkQueue {
                q: self.q.drain(0..ct).collect()
            }
        }).collect();
        assert_eq!(self.q.len(), 0);
        ret
    }

    pub fn retain(&mut self, mut f: impl FnMut(&N) -> bool) {
        for q in self.q.iter_mut() {
            q.retain(&mut f);
        }
    }

    pub fn append(&mut self, other: &mut ChunkQueue<N>) {
        self.q.append(&mut other.q);
    }
}
