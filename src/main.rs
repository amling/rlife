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
use gol::graph::GolNodeSerdeProxy;
use gol::graph::GolPreGraph;
use gol::graph::GolRecenter;
use gol::graph::GolSym;
use gol::lifecycle::GolLifecycle;

fn main() {
    main1::<u64>().unwrap();
}

fn main1<B: UScalar + DeserializeOwned + Serialize>() -> Result<(), StringError> {
    let ge: GolPreGraph = GolPreGraph {
        mt: 5,
        mx: 9,
        wx: 7,

        left_sym: GolSym::Empty,
        right_sym: GolSym::Empty,

        ox: 1,
        oy: 0,

        recenter: GolRecenter::BiasRight,
    };
    assert!(ge.mt * ge.mx <= B::size());
    let ge = ge.derived();

    let n0 = GolNodeSerdeProxy {
        dx: 0,
        r0: cnst(0b_000101000_000111000_000010000_000101000_000011000),
        r1: cnst(0b_000001000_000110000_000110000_000010000_000100000),
        r2: B::zero(),
        r2l: 0,
    };
    let n0 = n0.to_real(&ge);

    let re = DfsResToVec();

    let mut le = GolLifecycle {
        ge: &ge,
        threads: 8,
        recollect_ms: 5000,
        output_dir: None,
        log: None,
    };

    bfs::bfs2::<GolNode<B>, _, _, _, _>(n0, &ge, &re, &mut le);

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
