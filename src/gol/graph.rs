use std::marker::PhantomData;

use crate::bits;
use crate::dfs;

use bits::Bits;
use dfs::graph::DfsGraphConfig;

struct GolGraphEnv {
    period: usize,
    ox: usize,
    mx: usize,
    oy: usize,
}

struct GolGraphConfig<B> {
    _b: PhantomData<B>,
}

impl<B: Bits> DfsGraphConfig for GolGraphConfig<B> {
    type E = GolGraphEnv;
    type N = (B, B);

    fn start(e: &GolGraphEnv) -> (B, B) {
        assert!(e.period * e.mx <= B::size());
        (B::zero(), B::zero())
    }

    fn expand(e: &GolGraphEnv, n: &(B, B)) -> Vec<(B, B)> {
        unimplemented!();
    }

    fn end(e: &GolGraphEnv, n: &(B, B)) -> bool {
        unimplemented!();
    }
}
