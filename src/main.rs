#![allow(non_snake_case)]

#[macro_use]
extern crate ars_macro;

use ars_aa::lattice::LatticeCanonicalizable;
use ars_aa::lattice::LatticeCanonicalizer;
use ars_ds::err::StringError;
use ars_ds::scalar::UScalar;
use ars_rctl_main::rq::RctlRunQueue;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;

mod bfs;
mod chunk_store;
mod dedupe;
mod dfs;
mod gol;
mod lgol;
mod sal;

use bfs::bfs2::Bfs2CustomSerializer;
use bfs::bfs2::Bfs2State;
use chunk_store::AnonMmapChunkFactory;
use chunk_store::VecChunkFactory;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use gol::graph::GolEdge;
use gol::graph::GolGraphParams;
use gol::graph::GolHashNode;
use gol::graph::GolRecenter;
use gol::lifecycle::GolLifecycle;
use gol::lifecycle::GolRctlEp;
use gol::patlib::GolPatterns;
use lgol::axis::LGolBgEdge;
use lgol::axis::LGolEdgeRead;
use lgol::axis::LGolRecenteringAxis;
use lgol::axis::LGolReflectEdge;
use lgol::axis::LGolSimpleAxis;
use lgol::bg::LGolBgEmpty;
use lgol::bg::LGolBgHorizStripes;
use lgol::bg::LGolBgVertStripes;
use lgol::bg::LGolBgX2;
use lgol::bg::LGolBgY2;
use lgol::constraints::LGolConstraintUWindow;
use lgol::constraints::LGolConstraintVPeriodDividing;
use lgol::ends::LGolNoEnds;
use lgol::graph::LGolDedupeHack;
use lgol::graph::LGolGraphParams;
use lgol::lat1::Vec3;
use sal::DeserializerFor;

fn main() {
    let ep = Arc::new(GolRctlEp {
        threads: AtomicUsize::new(8),
        recollect_ms: AtomicU64::new(5000),
        max_mem: AtomicUsize::new(2 << 30),
        checkpt_rq: RctlRunQueue::new(),
    });

    ars_rctl_main::spawn(ep.clone());

    main1::<u16>(ep).unwrap();
}

fn main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -2, 3),
        vw: (0, -1, 2),

        bg_coord: PhantomData::<()>,

        u_axis: LGolRecenteringAxis {
            left_bg: LGolBgEmpty(),
            right_bg: LGolBgEmpty(),
        },
        v_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        constraints: (
            LGolConstraintUWindow {
                w: (wx, mx),
                left_bg: LGolBgEmpty(),
                right_bg: LGolBgEmpty(),
            },
        ),
    };
    let mut ge = ge.derived::<[B; 6], _>(HashSet::new());

    let cf = AnonMmapChunkFactory();
    let st: Bfs2State<_, _, LGolDedupeHack<HashSet<_>>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let rs = ge.parse_bs2(&[
            "   |   |...",
            ".*.|.*.|*.*",
            "*.*|.*.|   ",
            "z",
        ]);
        let (xyt, rs) = ge.recenter_xyt((0, 0, 0), rs);
        let n0 = ge.regular_node(xyt, rs);

        Bfs2State::new_simple(n0, cf)
    });

    {
        let rs = ge.parse_bs2(&[
            "    |    |.**.",
            "***.|..*.|..**",
            ".*.*|**.*|    ",
            "z",
        ]);
        let (xyt, rs) = ge.recenter_xyt((0, 0, 0), rs);
        let hn = ge.key_node(xyt, rs).lgol_hash_node();

        ge.ends.insert(hn);
    }

    assert!(ge.max_r1l <= B::size());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___bfs2___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();

    let ge = GolGraphParams {
        mt: 8,
        mx: mx,
        wx: wx,

        left_edge: GolEdge::Empty,
        right_edge: GolEdge::Empty,

        ox: 0,
        oy: 3,

        recenter: GolRecenter::BiasRight,
    };
    assert!(ge.mt * ge.mx <= B::size());

    let cf = VecChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let (r0, r1) = ge.parse_and_recenter_pair(
            "*..*. ..**. ..*.* .**.. *.*.. *.*.. ..**. .***.",
            "*...* ...** ..... .**.. .*... .*.*. *.... .*.*.",
        );
        let n0 = ge.regular_node::<B, ()>(r0, r1);

        Bfs2State::new_simple(n0, cf)
    });

    let ge = ge.derived((), ());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___bfs2___ends_db___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();

    let ge = GolGraphParams {
        mt: 8,
        mx: mx,
        wx: wx,

        left_edge: GolEdge::Empty,
        right_edge: GolEdge::Empty,

        ox: 0,
        oy: 3,

        recenter: GolRecenter::BiasRight,
    };
    assert!(ge.mt * ge.mx <= B::size());

    let cf = VecChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let (r0, r1) = ge.parse_and_recenter_pair(
            "*..*. ..**. ..*.* .**.. *.*.. *.*.. ..**. .***.",
            "*...* ...** ..... .**.. .*... .*.*. *.... .*.*.",
        );
        let n0 = ge.regular_node::<B, ()>(r0, r1);

        Bfs2State::new_simple(n0, cf)
    });

    let ends = {
        let mut ps = GolPatterns::new();
        ps.load("/home/amling/git/life-notes/ends");
        let mut ends = HashMap::new();
        let mut add_end = |r0, r1, label: &str| {
            ends.entry((r0, r1)).or_insert_with(|| Vec::new()).push(label.to_string());
        };
        add_end(B::zero(), B::zero(), "zero");
        for s in ps.find_slices(ge.ox, ge.oy, ge.mt) {
            if let Some((r0, r1)) = s.encode(ge.mx, ge.mt) {
                let (_, r0, r1) = ge.recenter(r0, r1);
                add_end(r0, r1, &s.label);
            }
        }
        ends
    };
    let ends: HashMap<_, _> = ends.into_iter().map(|((r0, r1), labels)| {
        (GolHashNode {
            r0: r0,
            r1: r1,
        }, labels.join(", "))
    }).collect();

    let ge = ge.derived((), ends);

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    le.log(LogLevel::INFO, format!("Loaded {} ends", ge.ends.len()));

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___lgol___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -1, 3),
        vw: (0, 0, 1),

        bg_coord: PhantomData::<()>,

        u_axis: LGolRecenteringAxis {
            left_bg: LGolBgEmpty(),
            right_bg: LGolBgEmpty(),
        },
        v_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        constraints: (
            LGolConstraintUWindow {
                w: (wx, mx),
                left_bg: LGolBgEmpty(),
                right_bg: LGolBgEmpty(),
            },
        ),
    };
    let ge = ge.derived::<[B; 6], _>(());

    let cf = VecChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let n0 = ge.zero_node();

        Bfs2State::new_simple(n0, cf)
    });

    assert!(ge.max_r1l <= B::size());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___lgol___oob_agar___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -1, 5),
        vw: (0, 0, 1),

        bg_coord: PhantomData::<LGolBgX2>,

        u_axis: LGolRecenteringAxis {
            left_bg: LGolBgVertStripes(),
            right_bg: LGolBgEmpty(),
        },
        v_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        constraints: (
            LGolConstraintUWindow {
                w: (wx, mx),
                left_bg: LGolBgVertStripes(),
                right_bg: LGolBgEmpty(),
            },
        ),
    };
    let mut ge = ge.derived::<[B; 10], _>(HashSet::new());

    let cf = VecChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let rs = ge.parse_bs2(&[
            "*...|*...|*...|*...|*...",
            "*...|*...|*...|*...|*...",
            "z",
        ]);
        let (xyt, rs) = ge.recenter_xyt((0, 0, 0), rs);
        let n0 = ge.regular_node(xyt, rs);

        Bfs2State::new_simple(n0, cf)
    });

    {
        let ends = vec![
            // ripped from main greyship
            &["**....", "*.*...", "*.....", "*.....", "*.....", "*.*...", "*.**..", "*.**..", "*.**..", "*.**.."],
            &["*.....", "*.....", "*.*...", "*.**..", "*.**..", "*.**..", "*.**..", "*.**..", "*.*...", "*.*.*."],
            &["*.*...", "*.**..", "*.**..", "*.**..", "*.**..", "*.**..", "*.*.*.", "*.*.*.", "*.*.*.", "*.*.*."],

            // 2c/2 wick edge, interesting if we could find it
            &["*.**..", "*.*.*.", "*.*...", "*..*..", "*.....", "*.**..", "*.**..", "*...*.", "*.*...", "*.*..."],
        ];
        for end in ends {
            let rs = ge.parse_bs(end);
            let (xyt, rs) = ge.recenter_xyt((0, 0, 0), rs);
            let n = ge.key_node(xyt, rs).lgol_hash_node();
            ge.ends.insert(n);
        }
    }

    assert!(ge.max_r1l <= B::size());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___lgol___periodic_edge___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let mt = args.parse();

    let vu = (0, 0, mt);
    let vv = (-2, 0, 3);

    let l2 = Vec3::canonicalize(vec![vu, vv]);
    let l2 = l2.materialize();
    assert_eq!(2, l2.len());
    let cvu = l2[1];
    let cvv = l2[0];

    let ge = LGolGraphParams {
        vu: cvu,
        vv: cvv,
        vw: (0, 1, 0),

        bg_coord: PhantomData::<()>,

        u_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        v_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        constraints: (),
    };
    let ge = ge.derived::<[B; 2], _>(());

    let cf = VecChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let n0 = ge.cb_node((0, 0, 0), |(x, _y, _t)| {
            x.rem_euclid(2) == 0
        });

        Bfs2State::new_simple(n0, cf)
    });

    assert!(ge.max_r1l <= B::size());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___lgol___reflect___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let mx = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -1, 3),
        vw: (0, 0, 1),

        bg_coord: PhantomData::<()>,

        u_axis: LGolSimpleAxis {
            left_edge: LGolBgEdge(LGolBgEmpty()),
            right_edge: LGolReflectEdge(0),
        },
        v_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        constraints: (),
    };
    let ge = ge.derived::<[B; 6], _>(());

    let cf = VecChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let n0 = ge.zero_node();

        Bfs2State::new_simple(n0, cf)
    });

    assert!(ge.max_r1l <= B::size());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

#[allow(dead_code)]
fn demo___lgol___period_divison___main1<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();
    let mf = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -4, 6),
        vw: (0, -1, 2),

        bg_coord: PhantomData::<LGolBgY2>,

        u_axis: LGolRecenteringAxis {
            left_bg: LGolBgHorizStripes(),
            right_bg: LGolBgEmpty(),
        },
        v_axis: (LGolEdgeRead::Wrap, LGolEdgeRead::Wrap),
        constraints: (
            LGolConstraintUWindow {
                w: (wx, mx),
                left_bg: LGolBgHorizStripes(),
                right_bg: LGolBgEmpty(),
            },
            LGolConstraintVPeriodDividing {
                division: 2,
                mf: mf,
            },
        ),
    };
    let ge = ge.derived::<[B; 6], _>(LGolNoEnds());

    let cf = AnonMmapChunkFactory();
    let st: Bfs2State<_, _, HashSet<_>> = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let rs = ge.parse_bs2(&[
            "     |     |     |     |     |..*..",
            "     |     |     |**...|**...|**...",
            "     |     |.....|.....|.....|     ",
            "**...|**...|**...|     |     |     ",
            "..*..|..*..|     |     |     |     ",
            "z",
        ]);
        let (xyt, rs) = ge.recenter_xyt((0, 0, 0), rs);
        let n0 = ge.regular_node(xyt, rs);

        Bfs2State::new_simple(n0, cf)
    });

    assert!(ge.max_r1l <= B::size());

    let mut le = GolLifecycle {
        ge: &ge,
        ep: ep,
    };

    bfs::bfs2(st, &ge, &mut le);

    le.log(LogLevel::INFO, "Done");

    Ok(())
}

struct ArgsHelper<I: Iterator<Item=String>>(I);

impl<I: Iterator<Item=String>> ArgsHelper<I> {
    fn parse<F: FromStr>(&mut self) -> F where F::Err: Debug {
        self.0.next().unwrap().parse().unwrap()
    }

    #[allow(dead_code)]
    fn parse_or<F: FromStr>(&mut self, def: F) -> F where F::Err: Debug {
        self.0.next().map(|s| s.parse().unwrap()).unwrap_or(def)
    }

    fn read_state_or<T>(&mut self, s: impl DeserializerFor<T>, f: impl FnOnce() -> T) -> T {
        match self.0.next() {
            Some(path) => {
                s.from_file(path).unwrap()
            }
            None => {
                f()
            }
        }
    }
}

fn env_args() -> ArgsHelper<impl Iterator<Item=String>> {
    ArgsHelper(std::env::args().skip(1))
}
