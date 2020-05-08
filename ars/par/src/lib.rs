use std::ops::Range;

use ars_ds::scalar::Scalar;
use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;

pub trait Par: Sized {
    type Item;
    type ItemIter: Iterator<Item=Self::Item>;
    type Chunk: Send;
    type ChunkIter: Iterator<Item=Self::Chunk>;

    fn par_split(self, ct: usize) -> Self::ChunkIter;
    fn iter_chunk(c: Self::Chunk) -> Self::ItemIter;

    fn go<TC: Default, CC: Default + Send, R: Send, F: Fn(Self::Item, &mut TC, &mut CC) -> R + Sync>(self, ct: usize, f: F) -> (Vec<R>, Vec::<CC>) {
        let cs: Vec<_> = self.par_split(ct * 10).collect();
        let mut ccs: Vec<_> = cs.iter().map(|_| CC::default()).collect();
        let mut rss: Vec<_> = cs.iter().map(|_| Vec::new()).collect();

        {
            let wq = SegQueue::new();
            for tuple in cs.into_iter().zip(ccs.iter_mut()).zip(rss.iter_mut()) {
                wq.push(tuple);
            }

            crossbeam::scope(|sc| {
                for _ in 0..ct {
                    sc.spawn(|_| {
                        let mut tc = TC::default();

                        loop {
                            let ((c, cc), rs) = match wq.pop() {
                                Ok(tuple) => tuple,
                                Err(PopError) => {
                                    return;
                                }
                            };

                            for i in Self::iter_chunk(c) {
                                let r = f(i, &mut tc, cc);
                                rs.push(r);
                            }
                        }
                    });
                }
            }).unwrap();
        }

        let rs = rss.into_iter().flatten().collect();

        (rs, ccs)
    }
}

struct VecRef<'a, T>(&'a Vec<T>);

impl<'a, T: Sync> Par for VecRef<'a, T> {
    type Item = &'a T;
    type ItemIter = std::slice::Iter<'a, T>;
    type Chunk = &'a [T];
    type ChunkIter = std::slice::Chunks<'a, T>;

    fn par_split(self, ct: usize) -> std::slice::Chunks<'a, T> {
        let sz = (self.0.len() + ct - 1) / ct;
        self.0.chunks(sz)
    }

    fn iter_chunk(c: &'a [T]) -> std::slice::Iter<'a, T> {
        c.iter()
    }
}

struct VecRefMut<'a, T>(&'a mut Vec<T>);

impl<'a, T: Send + Sync> Par for VecRefMut<'a, T> {
    type Item = &'a mut T;
    type ItemIter = std::slice::IterMut<'a, T>;
    type Chunk = &'a mut [T];
    type ChunkIter = std::slice::ChunksMut<'a, T>;

    fn par_split(self, ct: usize) -> std::slice::ChunksMut<'a, T> {
        let sz = (self.0.len() + ct - 1) / ct;
        self.0.chunks_mut(sz)
    }

    fn iter_chunk(c: &'a mut [T]) -> std::slice::IterMut<'a, T> {
        c.iter_mut()
    }
}

trait VecPar {
    type Item;

    fn par<'a>(&'a self) -> VecRef<'a, Self::Item>;
    fn par_mut<'a>(&'a mut self) -> VecRefMut<'a, Self::Item>;
}

impl<T> VecPar for Vec<T> {
    type Item = T;

    fn par<'a>(&'a self) -> VecRef<'a, T> {
        VecRef(self)
    }

    fn par_mut<'a>(&'a mut self) -> VecRefMut<'a, T> {
        VecRefMut(self)
    }
}

pub struct RangeItemIter<T>(T, T);

impl<T: Scalar> Iterator for RangeItemIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.0 == self.1 {
            return None;
        }
        let r = self.0;
        self.0 += T::one();
        Some(r)
    }
}

pub struct RangeChunkIter<T> {
    start: T,
    end: T,
    len: T,
    extra: T,
}

impl<T: Scalar> Iterator for RangeChunkIter<T> {
    type Item = RangeItemIter<T>;

    fn next(&mut self) -> Option<RangeItemIter<T>> {
        if self.start == self.end {
            debug_assert_eq!(self.extra, T::zero());
            return None;
        }

        let start = self.start;
        let mut len = self.len;
        if self.extra > T::zero() {
            len += T::one();
            self.extra -= T::one();
        }
        let end = start + len;
        debug_assert!(end <= self.end);
        self.start = end;

        Some(RangeItemIter(start, end))
    }
}

pub struct RangeOwn<T>(T, T);

impl<T: Scalar + Send> Par for RangeOwn<T> {
    type Item = T;
    type ItemIter = RangeItemIter<T>;
    type Chunk = RangeItemIter<T>;
    type ChunkIter = RangeChunkIter<T>;

    fn par_split(self, ct: usize) -> RangeChunkIter<T> {
        let ct = T::from_usize(ct);
        let len = (self.1 - self.0) / ct;
        let extra = self.1 - self.0 - len * ct;
        RangeChunkIter {
            start: self.0,
            end: self.1,
            len: len,
            extra: extra,
        }
    }

    fn iter_chunk(c: RangeItemIter<T>) -> RangeItemIter<T> {
        c
    }
}

pub trait RangePar {
    type Item;

    fn par(self) -> RangeOwn<Self::Item>;
}

impl<T: Scalar> RangePar for Range<T> {
    type Item = T;

    fn par(self) -> RangeOwn<T> {
        let mut start = self.start;
        if start > self.end {
            start = self.end;
        }
        RangeOwn(start, self.end)
    }
}
