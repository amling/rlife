#![allow(unused_parens)]

use serde::Deserialize;
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Deserialize)]
#[derive(Serialize)]
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
            if last.len() < last.capacity() {
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

    #[allow(dead_code)]
    pub fn into_iter(self) -> impl Iterator<Item=N> {
        self.q.into_iter().map(|q| q.into_iter()).flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item=&N> {
        self.q.iter().map(|q| q.iter()).flatten()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut N> {
        self.q.iter_mut().map(|q| q.iter_mut()).flatten()
    }

    #[allow(dead_code)]
    pub fn chunks_mut(&mut self) -> impl Iterator<Item=&mut [N]> {
        self.q.iter_mut().map(|q| {
            let (s1, s2) = q.as_mut_slices();
            vec![s1, s2]
        }).flatten()
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

    fn get_two_mut(&mut self, i: usize, j: usize) -> (&mut VecDeque<N>, &mut VecDeque<N>) {
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


    pub fn defragment(&mut self) {
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

            let space = p1.capacity() - p1.len();
            if space >= p2.len() {
                p1.append(p2);
                j += 1;
            }
            else {
                let mut q1 = std::mem::replace(p2, VecDeque::with_capacity(0));
                let q2 = q1.split_off(space);
                *p2 = q2;
                p1.append(&mut q1);
                i += 1;
            }
        }

        loop {
            if let Some(last) = self.q.back_mut() {
                if last.len() == 0 {
                    self.q.pop_back();
                    continue;
                }
            }

            break;
        }
    }

    pub fn append(&mut self, other: &mut ChunkQueue<N>) {
        self.q.append(&mut other.q);
        self.len += other.len;
        other.len = 0;
    }
}
