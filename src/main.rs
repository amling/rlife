#[macro_use]
extern crate ars_macro;

use ars_ds::err::StringError;
use ars_ds::scalar::UScalar;
use ars_rctl_main::rq::RctlRunQueue;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
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
use gol::lifecycle::GolLifecycle;
use gol::lifecycle::GolRctlEp;
use lgol::axis::LGolEdge;
use lgol::axis::LGolFancyAxis;
use lgol::bg::LGolBgEmpty;
use lgol::bg::LGolBgVertStripes;
use lgol::bg::LGolBgX2;
use lgol::graph::LGolGraphParams;
use sal::SerdeFormat;

fn main() {
    main1::<u16>().unwrap();
}

fn main1<B: UScalar + DeserializeOwned + Serialize>() -> Result<(), StringError> {
    let mut args = std::env::args().skip(1);

    let wx = args.next().unwrap().parse().unwrap();
    let mx = args.next().unwrap().parse().unwrap();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -1, 3),
        vw: (0, 0, 1),

        bg_coord: PhantomData::<LGolBgX2>,

        u_axis: LGolFancyAxis {
            w: (wx, mx),
            left_bg: LGolBgVertStripes(),
            right_bg: LGolBgEmpty(),
        },
        v_axis: (LGolEdge::Wrap, LGolEdge::Wrap),
    };
    let ge = ge.derived::<[B; 6]>();

    let st = match args.next() {
        Some(path) => {
            SerdeFormat::Bincode.read(path).unwrap()
        },
        None => {
            let rs = ge.parse_bs(&[
                "*...", "**..", "**..",
                "*...", "*...", "*.*.",
            ]);
            let (xyt, rs) = ge.recenter_xyt((0, 0, 0), rs);
            let n0 = ge.regular_node(xyt, rs);

            Bfs2State::new(vec![(vec![n0.key_node().unwrap()], n0)])
        },
    };

    assert!(ge.max_r1l <= B::size());

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

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}
