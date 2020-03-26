mod bfs;
mod bits;
mod dfs;
mod gol;

use bits::Bits;
use dfs::Tree;
use dfs::TreeStatus;
use dfs::res::DfsResToVec;
use gol::graph::GolGraph;
use gol::graph::GolNode;
use gol::graph::GolSym;
use gol::lifecycle::GolLifecycle;

fn main() {
    main1::<u128>();
}

fn main1<B: Bits>() {
    let ge = GolGraph {
        mt: 19,
        mx: 4,

        left_sym: GolSym::Gutter,
        right_sym: GolSym::Odd,

        ox: 0,
        oy: 0,
    };

    let re = DfsResToVec();

    let le = GolLifecycle {
        ge: &ge,
        threads: 8,
        recollect_ms: 1000,
    };

    assert!(ge.mt * ge.mx <= B::size());
    let n0 = GolNode {
        r0: B::cnst(0b0000010001000100110010000000001000010001010101010101110110010011001100100000),
        r1: B::cnst(0b0100010011001000000000100001000101010101010111011001001100110010000000000100),
        r2: B::zero(),
        r2l: 0,
    };
    let mut root = Tree(n0, TreeStatus::Unopened);

    dfs::dfs::<GolNode<B>, _, _, _, _>(&mut root, &ge, &re, &le);
}
