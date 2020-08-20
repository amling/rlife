use memmap::MmapMut;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::Index;
use std::ops::IndexMut;

pub trait ChunkRaw<T>: DerefMut<Target=[T]> + Send + Sync {
    fn cap(&self) -> usize;
}

pub trait ChunkFactory<T>: Copy + Send + Sync {
    type Output: ChunkRaw<T>;

    fn new_chunk(&self, min_size: usize) -> Self::Output;

    fn new_chunk_vec(&self, cap: usize) -> ChunkVec<T, Self::Output> {
        ChunkVec::new(self.new_chunk(cap))
    }

    fn new_chunk_vec_deque(&self, cap: usize) -> ChunkVecDeque<T, Self::Output> {
        ChunkVecDeque::new(self.new_chunk(cap))
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct VecChunkFactory();

impl<T: Send + Sync> ChunkRaw<T> for Vec<T> {
    fn cap(&self) -> usize {
        self.len()
    }
}

impl<T: Send + Sync + Default> ChunkFactory<T> for VecChunkFactory {
    type Output = Vec<T>;

    fn new_chunk(&self, min_size: usize) -> Vec<T> {
        let mut v = Vec::with_capacity(min_size);
        while v.len() < v.capacity() {
            v.push(T::default());
        }
        v
    }
}

// Careful, you take your life in your hands implementing this.
// TODO: deriver for this so people can avoid fucking it up
pub trait MmapChunkSafe: Copy + Default + Send + Sync {
}

macro_rules! impl_mmap_chunk_safe {
    ([] $t:tt) => {
        impl MmapChunkSafe for $t {
        }
    };
    ([$($p:ident),+] $t:tt) => {
        impl<$($p: MmapChunkSafe,)*> MmapChunkSafe for $t {
        }
    };
}

impl_mmap_chunk_safe!([] ());
impl_mmap_chunk_safe!([A] (A,));
impl_mmap_chunk_safe!([A, B] (A, B));
impl_mmap_chunk_safe!([A, B, C] (A, B, C));
impl_mmap_chunk_safe!([A, B, C, D] (A, B, C, D));
impl_mmap_chunk_safe!([A, B, C, D, E] (A, B, C, D, E));
impl_mmap_chunk_safe!([] u8);
impl_mmap_chunk_safe!([] u16);
impl_mmap_chunk_safe!([] u32);
impl_mmap_chunk_safe!([] u64);
impl_mmap_chunk_safe!([] usize);
impl_mmap_chunk_safe!([] isize);
impl_mmap_chunk_safe!([E] [E; 0]);
impl_mmap_chunk_safe!([E] [E; 1]);
impl_mmap_chunk_safe!([E] [E; 2]);
impl_mmap_chunk_safe!([E] [E; 3]);
impl_mmap_chunk_safe!([E] [E; 4]);
impl_mmap_chunk_safe!([E] [E; 5]);
impl_mmap_chunk_safe!([E] [E; 6]);
impl_mmap_chunk_safe!([E] [E; 7]);
impl_mmap_chunk_safe!([E] [E; 8]);
impl_mmap_chunk_safe!([E] [E; 9]);
impl_mmap_chunk_safe!([E] [E; 10]);
impl_mmap_chunk_safe!([E] [E; 11]);
impl_mmap_chunk_safe!([E] [E; 12]);
impl_mmap_chunk_safe!([E] [E; 13]);
impl_mmap_chunk_safe!([E] [E; 14]);
impl_mmap_chunk_safe!([E] [E; 15]);
impl_mmap_chunk_safe!([E] [E; 16]);
impl_mmap_chunk_safe!([E] [E; 17]);
impl_mmap_chunk_safe!([E] [E; 18]);
impl_mmap_chunk_safe!([E] [E; 19]);
impl_mmap_chunk_safe!([E] [E; 20]);
impl_mmap_chunk_safe!([E] [E; 21]);
impl_mmap_chunk_safe!([E] [E; 22]);
impl_mmap_chunk_safe!([E] [E; 23]);
impl_mmap_chunk_safe!([E] [E; 24]);
impl_mmap_chunk_safe!([E] [E; 25]);
impl_mmap_chunk_safe!([E] [E; 26]);
impl_mmap_chunk_safe!([E] [E; 27]);
impl_mmap_chunk_safe!([E] [E; 28]);
impl_mmap_chunk_safe!([E] [E; 29]);
impl_mmap_chunk_safe!([E] [E; 30]);
impl_mmap_chunk_safe!([E] [E; 31]);
impl_mmap_chunk_safe!([E] [E; 32]);

#[derive(Clone)]
#[derive(Copy)]
pub struct AnonMmapChunkFactory();

pub struct MmapChunk<T: MmapChunkSafe> {
    m: MmapMut,
    cap: usize,
    _t: PhantomData<T>,
}

impl<T: MmapChunkSafe> Deref for MmapChunk<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        let p: &[u8] = self.m.deref();
        let p: *const u8 = p.as_ptr();
        let p: *const T = p as *const T;
        let len = self.cap;
        unsafe {
            std::slice::from_raw_parts(p, len)
        }
    }
}

impl<T: MmapChunkSafe> DerefMut for MmapChunk<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        let p: &mut [u8] = self.m.deref_mut();
        let p: *mut u8 = p.as_mut_ptr();
        let p: *mut T = p as *mut T;
        let len = self.cap;
        unsafe {
            std::slice::from_raw_parts_mut(p, len)
        }
    }
}

impl<T: MmapChunkSafe> ChunkRaw<T> for MmapChunk<T> {
    fn cap(&self) -> usize {
        self.cap
    }
}

impl<T: MmapChunkSafe> ChunkFactory<T> for AnonMmapChunkFactory {
    type Output = MmapChunk<T>;

    fn new_chunk(&self, min_size: usize) -> MmapChunk<T> {
        let sz = std::mem::size_of::<T>();
        let req_size = min_size * sz;
        let m = MmapMut::map_anon(req_size).unwrap();
        let cap = m.deref().len() / sz;
        MmapChunk {
            m: m,
            cap: cap,
            _t: PhantomData,
        }
    }
}



pub struct ChunkVec<T, C: ChunkRaw<T>> {
    c: C,
    len: usize,
    _t: PhantomData<T>,
}

impl<T, C: ChunkRaw<T>> ChunkVec<T, C> {
    pub fn new(c: C) -> ChunkVec<T, C> {
        ChunkVec {
            c: c,
            len: 0,
            _t: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn cap(&self) -> usize {
        self.c.cap()
    }

    #[must_use]
    pub fn grow_one(&mut self) -> Option<&mut T> {
        if self.len >= self.cap() {
            return None;
        }
        let r = &mut self.c[self.len];
        self.len += 1;
        Some(r)
    }

    #[must_use]
    pub fn offer(&mut self, t: T) -> bool {
        match self.grow_one() {
            Some(p) => {
                *p = t;
                true
            }
            None => false,
        }
    }

    pub fn truncate(&mut self, len: usize) {
        self.len = self.len.min(len);
    }
}

impl<T, C: ChunkRaw<T>> Deref for ChunkVec<T, C> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        let c = self.c.deref();
        &c[0..self.len]
    }
}

impl<T, C: ChunkRaw<T>> DerefMut for ChunkVec<T, C> {
    fn deref_mut(&mut self) -> &mut [T] {
        let c = self.c.deref_mut();
        &mut c[0..self.len]
    }
}



pub struct ChunkVecDeque<T, C: ChunkRaw<T>> {
    c: C,
    off: usize,
    len: usize,
    _t: PhantomData<T>,
}

impl<T, C: ChunkRaw<T>> ChunkVecDeque<T, C> {
    pub fn new(c: C) -> ChunkVecDeque<T, C> {
        ChunkVecDeque {
            c: c,
            off: 0,
            len: 0,
            _t: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn grow_one(&mut self) -> Option<&mut T> {
        if self.len >= self.c.cap() {
            return None;
        }
        let i = self.len;
        self.len += 1;
        Some(&mut self[i])
    }

    #[must_use]
    pub fn offer(&mut self, t: T) -> bool {
        match self.grow_one() {
            Some(p) => {
                *p = t;
                true
            }
            None => false,
        }
    }

    fn as_slices_lens(&self) -> (usize, usize) {
        let cap = self.c.cap();
        if self.off + self.len <= cap {
            return (self.len, 0);
        }
        (cap - self.off, self.len - (cap - self.off))
    }

    pub fn as_slices(&self) -> (&[T], &[T]) {
        let (l1, l2) = self.as_slices_lens();
        let p = self.c.deref();
        let (s2, s1) = p.split_at(self.off);
        (&s1[0..l1], &s2[0..l2])
    }

    pub fn as_slices_mut(&mut self) -> (&mut [T], &mut [T]) {
        let (l1, l2) = self.as_slices_lens();
        let p = self.c.deref_mut();
        let (s2, s1) = p.split_at_mut(self.off);
        (&mut s1[0..l1], &mut s2[0..l2])
    }

    pub fn iter(&self) -> impl Iterator<Item=&T> {
        let (s1, s2) = self.as_slices();
        s1.iter().chain(s2.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut T> {
        let (s1, s2) = self.as_slices_mut();
        s1.iter_mut().chain(s2.iter_mut())
    }

    pub fn pop_front(&mut self) -> Option<T> where T: Copy {
        if self.len == 0 {
            return None;
        }
        let r = self.c[self.off];
        self.off = (self.off + 1) % self.c.cap();
        self.len -= 1;
        Some(r)
    }

    pub fn front(&self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }
        Some(&self.c[self.off])
    }

    pub fn retain(&mut self, f: &mut impl FnMut(&T) -> bool) where T: Copy {
        let mut i = 0;
        let mut j = 0;
        while j < self.len {
            if f(&self[j]) {
                self[i] = self[j];
                i += 1;
            }
            j += 1;
        }
        self.len = i;
    }

    pub fn shift_left(&mut self, other: &mut ChunkVecDeque<T, C>) where T: Copy {
        loop {
            // done
            if other.len == 0 {
                break;
            }
            // also done
            if self.len == self.c.cap() {
                break;
            }

            // figure out the "first" slice of other
            let from = {
                let end = (other.off + other.len).min(other.c.cap());
                // from other.off until either the end of its live contents or the end of its raw
                // buffer
                &other.c.deref()[other.off..end]
            };
            assert!(from.len() > 0);

            // figure out the "first" empty slice of ourselves
            let to = {
                let start = self.off + self.len;
                if start < self.c.cap() {
                    // we don't touch the back, there's space behind us
                    let cap = self.c.cap();
                    &mut self.c[start..cap]
                }
                else {
                    // we either touch the back or wrap around strictly, space runs up to our
                    // current head
                    let start = start - self.c.cap();
                    &mut self.c[start..self.off]
                }
            };
            assert!(to.len() > 0);

            // transfer as much as both slices will allow
            let ct = from.len().min(to.len());
            let from = &from[0..ct];
            let to = &mut to[0..ct];
            to.copy_from_slice(from);

            // update heads and lengths to include the copy
            self.len += ct;
            other.len -= ct;
            other.off = (other.off + ct) % other.c.cap();

            // and rather, linse, repeat
        }
    }
}

impl<T, C: ChunkRaw<T>> Index<usize> for ChunkVecDeque<T, C> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        debug_assert!(i < self.len);
        let i = (self.off + i) % self.c.cap();
        &self.c[i]
    }
}

impl<T, C: ChunkRaw<T>> IndexMut<usize> for ChunkVecDeque<T, C> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        debug_assert!(i < self.len);
        let i = (self.off + i) % self.c.cap();
        &mut self.c[i]
    }
}
