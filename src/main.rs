#[macro_use]
extern crate ars_macro;

use ars_ds::err::StringError;
use ars_ds::scalar::UScalar;
use ars_rctl_main::rq::RctlRunQueue;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;

mod bfs;
mod dfs;
mod gol;
mod lgol;
mod sal;

use bfs::bfs2::Bfs2State;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use gol::graph::GolEdge;
use gol::graph::GolGraphParams;
use gol::graph::GolNode;
use gol::graph::GolRecenter;
use gol::lifecycle::GolLifecycle;
use gol::lifecycle::GolRctlEp;
use sal::SerdeFormat;

fn main() {
    main1::<u64>().unwrap();
}

fn main1<B: UScalar + DeserializeOwned + Serialize>() -> Result<(), StringError> {
    let mut args = std::env::args().skip(1);

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

    let st = match args.next() {
        Some(path) => {
            SerdeFormat::Bincode.read(path).unwrap()
        },
        None => {
            let n0 = ge.zero_node::<B, ()>();

            Bfs2State::new(vec![(vec![n0.key_node().unwrap()], n0)])
        },
    };

    let ge = ge.derived((), ());

    let ep = Arc::new(GolRctlEp {
        threads: AtomicUsize::new(8),
        recollect_ms: AtomicU64::new(5000),
        max_mem: AtomicUsize::new(2 << 30),
        checkpt_rq: RctlRunQueue::new(),
    });

    ars_rctl_main::spawn(ep.clone());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2::<GolNode<B, _>, _, _>(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}
