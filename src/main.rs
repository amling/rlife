#[macro_use]
extern crate ars_macro;

use ars_ds::err::StringError;
use ars_ds::scalar::UScalar;
use chrono::Local;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::path::Path;

mod bfs;
mod dfs;
mod gol;

use dfs::Tree;
use dfs::TreeStatus;
use dfs::res::DfsResToVec;
use gol::graph::GolNode;
use gol::graph::GolPreGraph;
use gol::graph::GolRecenter;
use gol::graph::GolSym;
use gol::lifecycle::GolLifecycle;

fn main() {
    main1::<u64>().unwrap();
}

fn main1<B: UScalar + DeserializeOwned + Serialize>() -> Result<(), StringError> {
    let dir = std::env::args().skip(1).next().unwrap();
    std::fs::create_dir_all(&dir)?;

    let ge: GolPreGraph = load_or_with(&dir, "ge", || {
        GolPreGraph {
            mt: 3,
            mx: 10,
            wx: 10,

            left_sym: GolSym::Empty,
            right_sym: GolSym::Empty,

            ox: 0,
            oy: 1,

            recenter: GolRecenter::BiasRight,
        }
    })?;
    assert!(ge.mt * ge.mx <= B::size());
    let ge = ge.derived();

    let mut root = load_or_with(&dir, "tree", || {
        let n0 = GolNode {
            dx: 0,
            r0: B::zero(),
            r1: B::zero(),
            r2: B::zero(),
            r2l: 0,
            min_x: ge.mx - 1,
            max_x: 0,
        };
        Tree(n0, TreeStatus::Unopened).to_serde_proxy()
    })?.to_tree();

    let re = DfsResToVec();

    let log = format!("{}/log.{}", dir, Local::now().format("%Y%m%d-%H%M%S"));

    let mut le = GolLifecycle {
        ge: &ge,
        threads: 8,
        recollect_ms: 1000,
        output_dir: Some(dir.clone()),
        log: Some(File::create(log)?),
    };

    dfs::dfs::<GolNode<B>, _, _, _, _>(&mut root, &ge, &re, &mut le);

    Ok(())
}

fn load_or_with<T: DeserializeOwned + Serialize>(dir: impl AsRef<str>, file: impl AsRef<str>, init: impl FnOnce() -> T) -> Result<T, StringError> {
    let dir = dir.as_ref();
    let file = file.as_ref();

    let path = format!("{}/{}", dir, file);
    let path = Path::new(&path);
    if path.is_file() {
        let f = File::open(path)?;
        let f = BufReader::new(f);
        Ok(serde_json::from_reader(f)?)
    }
    else {
        let r = init();
        let f = File::create(path)?;
        let f = BufWriter::new(f);
        serde_json::to_writer(f, &r)?;
        Ok(r)
    }
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
