use ars_ds::scalar::UScalar;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::gol;

use gol::graph::GolHashNode;
use gol::graph::GolKeyNode;

pub trait GolEnds<B: UScalar> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&str>;
}

impl<B: UScalar> GolEnds<B> for () {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&str> {
        if n.r0 == B::zero() && n.r1 == B::zero() {
            Some("")
        }
        else {
            None
        }
    }
}

impl<B: UScalar> GolEnds<B> for HashSet<GolHashNode<B>> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&str> {
        if self.contains(&n.gol_hash_node()) {
            Some("")
        }
        else {
            None
        }
    }
}

impl<B: UScalar, S: AsRef<str>> GolEnds<B> for HashMap<GolHashNode<B>, S> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&str> {
        self.get(&n.gol_hash_node()).map(|s| s.as_ref())
    }
}
