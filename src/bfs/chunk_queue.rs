#![allow(unused_parens)]

use std::collections::VecDeque;

use crate::chunk_store;

use chunk_store::ChunkFactory;
use chunk_store::ChunkVecDeque;

pub struct ChunkQueue<N, CF: ChunkFactory<N>> {
    pub cf: CF,
    len: usize,
    q: VecDeque<ChunkVecDeque<N, CF::Output>>,
}

impl<N, CF: ChunkFactory<N>> ChunkQueue<N, CF> {
    fn chunk_size(&self) -> usize {
        let d = std::mem::size_of::<N>();
        (1 << 20) / d
    }

    pub fn new(cf: CF) -> Self {
        ChunkQueue {
            cf: cf,
            len: 0,
            q: VecDeque::new(),
        }
    }

    pub fn push_back(&mut self, n: N) where N: Copy {
        self.len += 1;

        if let Some(last) = self.q.back_mut() {
            if last.offer(n) {
                return;
            }
        }
        self.q.push_back(self.cf.new_chunk_vec_deque(self.chunk_size()));
        assert!(self.q.back_mut().unwrap().offer(n));
    }

    pub fn len(&self) -> usize {
        debug_assert_eq!(self.len, self.q.iter().map(|q| q.len()).sum::<usize>());
        self.len
    }

    pub fn pop_front(&mut self) -> Option<N> where N: Copy {
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

    pub fn drain_partition(&mut self, shards: usize) -> Vec<ChunkQueue<N, CF>> {
        let len = self.q.len();
        let ret = (0..shards).map(|i| {
            let ct = ((i + 1) * len / shards - i * len / shards);
            let q: VecDeque<_> = self.q.drain(0..ct).collect();
            ChunkQueue {
                cf: self.cf,
                len: q.iter().map(|q| q.len()).sum(),
                q: q,
            }
        }).collect();
        self.len = 0;
        assert_eq!(self.q.len(), 0);
        ret
    }

    pub fn retain(&mut self, mut f: impl FnMut(&N) -> bool) where N: Copy {
        let mut len = 0;
        for q in self.q.iter_mut() {
            q.retain(&mut f);
            len += q.len();
        }
        self.len = len;
    }

    fn get_two_mut(&mut self, i: usize, j: usize) -> (&mut ChunkVecDeque<N, CF::Output>, &mut ChunkVecDeque<N, CF::Output>) {
        let (s1, s2) = self.q.as_mut_slices();

        let s1l = s1.len();
        if i < s1l {
            if j < s1l {
                let (s1, s2) = s1.split_at_mut(j);
                (&mut s1[i], &mut s2[0])
            }
            else {
                (&mut s1[i], &mut s2[j - s1l])
            }
        }
        else {
            if j < s1l {
                panic!();
            }
            else {
                let i = i - s1l;
                let j = j - s1l;
                let (s1, s2) = s2.split_at_mut(j);
                let s1l = s1.len();

                (&mut s1[i], &mut s2[j - s1l])
            }
        }
    }


    pub fn defragment(&mut self) where N: Copy {
        let mut i = 0;
        let mut j = 0;

        loop {
            if i == j {
                j += 1;
            }
            if j >= self.q.len() {
                break;
            }

            let (p1, p2) = self.get_two_mut(i, j);

            p1.shift_left(p2);
            if p2.len() == 0 {
                j += 1;
            }
            else {
                i += 1;
            }
        }

        // We are/were pushing into i, but i could be empty or off the end of the vec, so be
        // careful...
        if i < self.q.len() && self.q[i].len() > 0 {
            i += 1;
        }
        self.q.truncate(i);
    }

    pub fn append(&mut self, other: &mut ChunkQueue<N, CF>) {
        self.q.append(&mut other.q);
        self.len += other.len;
        other.len = 0;
    }
}
