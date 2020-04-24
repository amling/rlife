#![allow(unused_parens)]

use std::collections::VecDeque;

pub struct ChunkQueue<N> {
    len: usize,
    q: VecDeque<VecDeque<N>>,
}

impl<N> ChunkQueue<N> {
    fn chunk_size(&self) -> usize {
        let d = std::mem::size_of::<N>();
        ((1 << 20) + d) / d
    }

    pub fn new() -> Self {
        ChunkQueue {
            len: 0,
            q: VecDeque::new(),
        }
    }

    pub fn push_back(&mut self, n: N) {
        self.len += 1;

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
        debug_assert_eq!(self.len, self.q.iter().map(|q| q.len()).sum::<usize>());
        self.len
    }

    pub fn pop_front(&mut self) -> Option<N> {
        loop {
            match self.q.front_mut() {
                Some(q) => {
                    match q.pop_front() {
                        Some(n) => {
                            self.len -= 1;
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
            let q: VecDeque<_> = self.q.drain(0..ct).collect();
            ChunkQueue {
                len: q.iter().map(|q| q.len()).sum(),
                q: q,
            }
        }).collect();
        self.len = 0;
        assert_eq!(self.q.len(), 0);
        ret
    }

    pub fn retain(&mut self, mut f: impl FnMut(&N) -> bool) {
        let mut len = 0;
        for q in self.q.iter_mut() {
            q.retain(&mut f);
            len += q.len();
        }
        self.len = len;
    }

    pub fn append(&mut self, other: &mut ChunkQueue<N>) {
        self.q.append(&mut other.q);
        self.len += other.len;
        other.len = 0;
    }
}
