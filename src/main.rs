mod bits;
mod dfs;
mod gol;

use bits::Bits;
use dfs::res::DfsResToVec;
use gol::graph::GolGraph;
use gol::lifecycle::GolLifecycle;

fn main() {
    main1::<u64>();
}

fn main1<B: Bits>() {
    let ge = GolGraph {
        mt: 4,
        mx: 6,

        ox: 0,
        oy: 1,
    };

    let re = DfsResToVec();

    let le = GolLifecycle {
        ge: &ge,
        threads: 8,
        recollect_ms: 1000,
    };

    dfs::dfs::<(B, B, B, usize), _, _, _, _>(&ge, &re, &le);
}
