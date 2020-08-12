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
use std::io::BufRead;
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;

mod bfs;
mod chunk_store;
mod dfs;
mod gol;
mod lgol;
mod sal;

use bfs::bfs2::Bfs2CustomSerializer;
use bfs::bfs2::Bfs2State;
use chunk_store::AnonMmapChunkFactory;
use chunk_store::VecChunkFactory;
use dfs::graph::DfsNode;
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
use lgol::graph::LGolGraphParams;
use lgol::graph::LGolHashNode;
use lgol::graph::LGolKeyNode;
use lgol::graph::LGolNode;
use lgol::lat1::Vec3;
use sal::DeserializerFor;

fn main() {
    let ep = Arc::new(GolRctlEp {
        threads: AtomicUsize::new(12),
        recollect_ms: AtomicU64::new(5000),
        max_mem: AtomicUsize::new(24 << 30),
        checkpt_rq: RctlRunQueue::new(),
    });

    ars_rctl_main::spawn(ep.clone());

    if std::env::var("RLIFE_ANA").is_ok() {
        main1_ana::<u16>().unwrap();
    }
    else {
        main1_srch::<u16>(ep).unwrap();
    }
}

fn main1_srch<B: UScalar + DeserializeOwned + Serialize>(ep: Arc<GolRctlEp>) -> Result<(), StringError> {
    let mut args = env_args();

    let jx = args.parse();
    let wx = args.parse();
    let mx = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -2, 5),
        vw: (0, -1, 3),

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
        ),
    };
    let mut ge = ge.derived::<[B; 10], _>(HashSet::new());

    let mut hns = HashSet::new();
    for dy in 0..=1 {
        for bits in 0..(1u64 << (10 * jx)) {
            let mut rs = [B::zero(); 10];
            for i in 0..10 {
                for x in 0..jx {
                    if bits & (1 << (jx * i + x)) != 0 {
                        rs[i].set_bit(x, true);
                    }
                }
            }

            let hn = LGolHashNode {
                bg_coord: LGolBgY2(dy),
                rs: rs,
            };
            let (_, _, hn) = ge.recenter(hn);

            hns.insert(hn);
        }
    }

    let cf = AnonMmapChunkFactory();
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
        let init = hns.iter().map(|hn| {
            let y = hn.bg_coord.0 as isize;
            let n = ge.regular_node((0, y, 0), hn.rs);
            (vec![n.key_node().unwrap()], n)
        });

        Bfs2State::new(init, cf)
    });

    for hn in hns.iter() {
        ge.ends.insert(hn.clone());
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

fn main1_ana<B: UScalar + DeserializeOwned + Serialize>() -> Result<(), StringError> {
    let mut args = env_args();

    let wx = args.parse();
    let mx = args.parse();

    let ge = LGolGraphParams {
        vu: (mx, 0, 0),
        vv: (0, -2, 5),
        vw: (0, -1, 3),

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
        ),
    };
    let ge = ge.derived::<[B; 10], _>(());

    let mut fw = HashMap::new();
    let mut bw = HashMap::new();
    let mut living = HashSet::new();
    {
        let t0 = std::time::Instant::now();
        for line in std::io::stdin().lock().lines() {
            let line = line.unwrap();

            let extract = |p: &'static str| {
                line.find(p).map(|idx| &line[(idx + p.len())..])
            };

            if let Some(end) = extract("End JSON: ") {
                let path: Vec<LGolKeyNode<[B; 10], LGolBgY2>> = serde_json::from_str(end)?;

                for i in 0..(path.len() - 1) {
                    let n1 = &path[i];
                    let n2 = &path[i + 1];
                    let ushift = n2.du - n1.du;
                    let vshift = n2.dv - n1.dv;
                    let n1 = n1.lgol_hash_node();
                    let n2 = n2.lgol_hash_node();
                    fw.entry(n1.clone()).or_insert_with(|| HashSet::new()).insert((n2.clone(), (ushift, vshift)));
                    bw.entry(n2.clone()).or_insert_with(|| HashSet::new()).insert((n1.clone(), (-ushift, -vshift)));
                    living.insert(n1.clone());
                    living.insert(n2.clone());
                }
            }
        }
        eprintln!("Read input in {:?}", t0.elapsed());
    }

    loop {
        eprintln!("Loop: {} links, {} nodes", fw.iter().map(|(_, s)| s.len()).sum::<usize>(), living.len());

        {
            let t0 = std::time::Instant::now();
            if living.len() < 10 {
                for n0 in living.iter() {
                    eprintln!("n0");
                    let kn = LGolKeyNode {
                        bg_coord: n0.bg_coord,
                        du: 0,
                        dv: 0,
                        rs: n0.rs,
                    };
                    for line in ge.format_rows(&vec![kn], None) {
                        eprintln!("   {}", line);
                    }
                }
            }
            let ct0 = living.len();
            for &m in &[&fw, &bw] {
                living.retain(|n| m.get(n).map(|s| s.len()).unwrap_or(0) > 0);
            }
            eprintln!("Pruned living in {:?}", t0.elapsed());
            if living.len() == ct0 {
                break;
            }
        }

        {
            let prune_m = |m: &mut HashMap<LGolHashNode<[B; 10], LGolBgY2>, HashSet<(LGolHashNode<[B; 10], LGolBgY2>, (i16, i16))>>, label| {
                let t0 = std::time::Instant::now();
                m.retain(|n, _| living.contains(n));
                for (_, s) in m.iter_mut() {
                    s.retain(|(n, _)| living.contains(n));
                }
                eprintln!("Pruned {} in {:?}", label, t0.elapsed());
            };
            prune_m(&mut fw, "fw");
            prune_m(&mut bw, "bw");
        }
    }
    eprintln!("Done pruning");

    let n0 = living.iter().next().unwrap().clone();

    let (path, cycle, last) = (|| {
        let mut path = dfs::Path::<LGolNode<[B; 10], LGolBgY2, ()>>::new();
        let mut du = 0;
        let mut dv = 0;
        let mut n = n0.clone();
        loop {
            let kn = LGolKeyNode {
                bg_coord: n.bg_coord,
                du: du,
                dv: dv,
                rs: n.rs,
            };
            if let Some(idx) = path.find_or_push(&kn) {
                return ((&path.vec[0..idx]).to_vec(), (&path.vec[idx..]).to_vec(), kn);
            }

            let (n2, (ushift, vshift)) = fw.get(&n).unwrap().iter().next().unwrap();

            n = n2.clone();
            du += ushift;
            dv += vshift;
        }
    })();

    for line in ge.format_cycle_rows(&path, &cycle, &last) {
        eprintln!("{}", line);
    }

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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
    let st = args.read_state_or(Bfs2CustomSerializer(cf), || {
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
