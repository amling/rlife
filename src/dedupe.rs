use std::collections::HashSet;
use std::hash::Hash;

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
