use ars_ds::scalar::UScalar;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::gol;

use gol::graph::GolHashNode;
use gol::graph::GolKeyNode;

pub trait GolEnds<B: UScalar> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&'static str>;
}

impl<B: UScalar> GolEnds<B> for () {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&'static str> {
        if n.r0 == B::zero() && n.r1 == B::zero() {
            Some("")
        }
        else {
            None
        }
    }
}

impl<B: UScalar> GolEnds<B> for HashSet<GolHashNode<B>> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&'static str> {
        if self.contains(&n.gol_hash_node()) {
            Some("")
        }
        else {
            None
        }
    }
}

impl<B: UScalar> GolEnds<B> for HashMap<GolHashNode<B>, &'static str> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&'static str> {
        self.get(&n.gol_hash_node()).map(|s| *s)
    }
}
