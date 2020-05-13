use ars_ds::scalar::UScalar;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::gol;

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

impl<B: UScalar> GolEnds<B> for HashSet<(B, B)> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&'static str> {
        if self.contains(&(n.r0, n.r1)) {
            Some("")
        }
        else {
            None
        }
    }
}

impl<B: UScalar> GolEnds<B> for HashMap<(B, B), &'static str> {
    fn end(&self, n: &GolKeyNode<B>) -> Option<&'static str> {
        self.get(&(n.r0, n.r1)).map(|s| *s)
    }
}
