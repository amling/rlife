#[macro_use]
extern crate ars_macro;

use ars_ds::err::StringError;
use ars_ds::scalar::UScalar;
use ars_rctl_main::rq::RctlRunQueue;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

mod bfs;
mod dfs;
mod gol;
mod sal;

use dfs::graph::DfsNode;
use gol::graph::GolEdge;
use gol::graph::GolGraphParams;
use gol::graph::GolNode;
use gol::graph::GolNodeSerdeProxy;
use gol::graph::GolRecenter;
use gol::lifecycle::GolLifecycle;
use gol::lifecycle::GolRctlEp;

fn main() {
    main1::<u64>().unwrap();
}

fn main1<B: UScalar + DeserializeOwned + Serialize>() -> Result<(), StringError> {
    let ge = GolGraphParams {
        mt: 3,
        mx: 8,
        wx: 8,

        left_edge: GolEdge::Empty,
        right_edge: GolEdge::Empty,

        ox: 0,
        oy: 1,

        recenter: GolRecenter::BiasRight,
    };
    assert!(ge.mt * ge.mx <= B::size());
    let ge = ge.derived((), ());

    let n0 = GolNodeSerdeProxy {
        dx: 0,
        dy: (),
        r0: B::zero(),
        r1: B::zero(),
        r2: B::zero(),
        r2l: 0,
    };
    let n0 = ge.params.thaw_node(&n0);

    let (shift, _, _) = ge.params.recenter(n0.r0, n0.r1);
    assert_eq!(0, shift);

    let ep = Arc::new(GolRctlEp {
        threads: 8,
        recollect_ms: 5000,
        max_mem: AtomicUsize::new(8 << 30),
        checkpt_rq: RctlRunQueue::new(),
    });

    ars_rctl_main::spawn(ep.clone());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2::<GolNode<B, _>, _, _>(vec![(vec![n0.key_node().unwrap()], n0)], &ge, &mut le);

    Ok(())
}

fn cnst<B: UScalar>(c: u128) -> B {
    let mut b = B::zero();
    let mut c = c;
    let mut idx = 0;
    while c > 0 {
        if c % 2 == 1 {
            assert!(idx < B::size());
            b.set_bit(idx, true);
        }
        c >>= 1;
        idx += 1;
    }
    b
}
