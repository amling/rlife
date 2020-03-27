extern crate serde;

use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

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
    let dir = std::env::args().skip(1).next().unwrap();
    std::fs::create_dir_all(&dir).unwrap();

    let ge: GolGraph = load_or_with(&dir, "ge", || {
        GolGraph {
            mt: 19,
            mx: 4,

            left_sym: GolSym::Gutter,
            right_sym: GolSym::Odd,

            ox: 0,
            oy: 0,
        }
    });
    assert!(ge.mt * ge.mx <= B::size());

    let mut root = load_or_with(&dir, "tree", || {
        let n0 = GolNode {
            r0: B::cnst(0b0000010001000100110010000000001000010001010101010101110110010011001100100000),
            r1: B::cnst(0b0100010011001000000000100001000101010101010111011001001100110010000000000100),
            r2: B::zero(),
            r2l: 0,
        };
        Tree(n0, TreeStatus::Unopened).to_serde_proxy()
    }).to_tree();

    let re = DfsResToVec();

    let mut le = GolLifecycle {
        ge: &ge,
        threads: 8,
        recollect_ms: 1000,
        output_dir: Some(dir.clone()),
    };

    dfs::dfs::<GolNode<B>, _, _, _, _>(&mut root, &ge, &re, &mut le);
}

fn load_or_with<T: DeserializeOwned + Serialize>(dir: impl AsRef<str>, file: impl AsRef<str>, init: impl FnOnce() -> T) -> T {
    let dir = dir.as_ref();
    let file = file.as_ref();

    let path = format!("{}/{}", dir, file);
    let path = Path::new(&path);
    if path.is_file() {
        let mut f = File::open(path).unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        serde_json::from_str(&s).unwrap()
    }
    else {
        let r = init();
        let mut f = File::create(path).unwrap();
        let s = serde_json::to_string(&r).unwrap();
        f.write_all(s.as_bytes()).unwrap();
        r
    }
}
