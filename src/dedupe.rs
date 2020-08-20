use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use crate::chunk_store;

use chunk_store::ChunkFactory;
use chunk_store::ChunksVec;

pub trait Dedupe<E, CF> {
    fn new(cf: CF) -> Self;
    fn len(&self) -> usize;
    fn cloned_iter<'a>(&'a self) -> Box<dyn Iterator<Item=E> + 'a>;
    fn insert(&mut self, e: E) -> bool;
}

impl<E: Clone + Hash + Eq, CF> Dedupe<E, CF> for HashSet<E> {
    fn new(_cf: CF) -> Self {
        HashSet::new()
    }

    fn len(&self) -> usize {
        HashSet::len(self)
    }

    fn cloned_iter<'a>(&'a self) -> Box<dyn Iterator<Item=E> + 'a> {
        Box::new(HashSet::iter(self).cloned())
    }

    fn insert(&mut self, e: E) -> bool {
        HashSet::insert(self, e)
    }
}

pub struct CfHashSet<E, CF: ChunkFactory<usize> + ChunkFactory<(E, usize)>> {
    table: ChunksVec<usize, CF>,
    nodes: ChunksVec<(E, usize), CF>,
}

impl<E, CF: ChunkFactory<usize> + ChunkFactory<(E, usize)>> CfHashSet<E, CF> {
    fn hash(&self, e: &E) -> usize where E: Hash {
        let mut s = DefaultHasher::new();
        e.hash(&mut s);
        s.finish() as usize
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item=&'a E> {
        let nodes = &self.nodes;
        self.table.iter().flat_map(move |&p| {
            CfHashSetWalk {
                p: p,
                nodes: nodes,
            }
        })
    }

    fn maybe_rehash(&mut self) where E: Hash {
        if self.nodes.len() <= self.table.len() {
            return;
        }

        let new_size = self.table.len() * 11 / 10;
        for _ in self.table.len()..new_size {
            self.table.push(0);
        }

        // Rehash each bucket.  It doesn't matter if we push stuff right, it will just get rehashed
        // a second time (back into same bucket).
        for i in 0..new_size {
            // take the entire chain out of this bucket
            let mut p = self.table[i];
            self.table[i] = 0;

            // loop invariant: we own the chain at p and beyond and everything else is linked up
            while p != 0 {
                let (ref e, p2) = self.nodes[p];

                let hash = self.hash(e);
                let bucket = hash % self.table.len();

                self.nodes[p].1 = self.table[bucket];
                self.table[bucket] = p;

                p = p2;
            }
        }
    }
}

impl<E: Clone + Hash + Eq + Default, CF: ChunkFactory<usize> + ChunkFactory<(E, usize)>> Dedupe<E, CF> for CfHashSet<E, CF> {
    fn new(cf: CF) -> Self {
        let table_size = 10;
        let mut table = ChunksVec::new(cf);
        for _ in 0..table_size {
            table.push(0);
        }

        let mut nodes = ChunksVec::new(cf);
        nodes.push((E::default(), 0));

        CfHashSet {
            table: table,
            nodes: nodes,
        }
    }

    fn len(&self) -> usize {
        self.nodes.len() - 1
    }

    fn cloned_iter<'a>(&'a self) -> Box<dyn Iterator<Item=E> + 'a> {
        Box::new(self.iter().cloned())
    }

    fn insert(&mut self, e: E) -> bool {
        let hash = self.hash(&e);
        let bucket = hash % self.table.len();
        let p0 = self.table[bucket];
        let mut p = p0;
        while p != 0 {
            let (ref e2, p2) = self.nodes[p];
            if &e == e2 {
                // conceivably we could benefit from rotating this to the front to LRU optimize
                return false;
            }
            p = p2;
        }

        let p = self.nodes.push((e, p0));
        self.table[bucket] = p;

        self.maybe_rehash();

        true
    }
}

struct CfHashSetWalk<'a, E, CF: ChunkFactory<(E, usize)>> {
    nodes: &'a ChunksVec<(E, usize), CF>,
    p: usize,
}

impl<'a, E, CF: ChunkFactory<(E, usize)>> Iterator for CfHashSetWalk<'a, E, CF> {
    type Item = &'a E;

    fn next(&mut self) -> Option<&'a E> {
        if self.p == 0 {
            return None;
        }
        let (ref e, p2) = self.nodes[self.p];
        self.p = p2;
        Some(e)
    }
}
