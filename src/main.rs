mod bits;
mod dfs;
mod gol;

use bits::Bits;
use dfs::res::DfsResToVec;
use gol::graph::GolGraph;
use gol::lifecycle::GolLifecycle;

fn main() {
    main1::<u32>();
}

fn main1<B: Bits>() {
    let ge = GolGraph {
        mt: 1,
        mx: 3,

        ox: 0,
        oy: 0,
    };

    let re = DfsResToVec();

    let le = GolLifecycle {
        ge: &ge,
        threads: 4,
        recollect_ms: 1000,
    };

    dfs::sdfs::<(B, B), _, _, _, _>(&ge, &re, &le);
}
