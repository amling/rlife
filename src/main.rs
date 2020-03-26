mod bfs;
mod bits;
mod dfs;
mod gol;

use bits::Bits;
use dfs::res::DfsResToVec;
use gol::graph::GolGraph;
use gol::graph::GolNode;
use gol::lifecycle::GolLifecycle;

fn main() {
    main1::<u64>();
}

fn main1<B: Bits>() {
    let ge = GolGraph {
        mt: 1,
        mx: 19,

        ox: 7,
        oy: 0,
    };

    let re = DfsResToVec();

    let le = GolLifecycle {
        ge: &ge,
        threads: 8,
        recollect_ms: 1000,
    };

    bfs::sbfs::<GolNode<B>, _, _, _, _>(&ge, &re, &le);
}
