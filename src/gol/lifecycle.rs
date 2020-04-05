use ars_ds::bit_state::Bits;
use serde::Serialize;
use std::fs::File;
use std::io::Write;

use crate::dfs;
use crate::gol;

use dfs::Tree;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsResVec;
use gol::graph::GolGraph;
use gol::graph::GolKeyNode;
use gol::graph::GolNode;

pub struct GolLifecycle<'a> {
    pub ge: &'a GolGraph,
    pub threads: usize,
    pub recollect_ms: u64,
    pub output_dir: Option<String>,
    pub log: Option<File>,
}

impl<'a> GolLifecycle<'a> {
    fn log(&mut self, s: impl Into<String>) {
        let s = s.into();
        if let Some(log) = &mut self.log {
            writeln!(log, "{}", s).unwrap();
        }
        else {
            println!("{}", s);
        }
    }
}

impl<'a, B: Bits + Serialize> DfsLifecycle<GolNode<B>, DfsResVec<GolKeyNode<B>>> for GolLifecycle<'a> {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
    }

    fn on_recollect_firstest(&mut self, firstest: Vec<GolKeyNode<B>>) {
        eprintln!("Recollect firstest...");
        for line in self.ge.format_rows(&firstest) {
            eprintln!("{}", line);
        }
    }

    fn on_recollect_results(&mut self, r: DfsResVec<GolKeyNode<B>>) -> bool {
        for cycle in &r.cycles {
            let (path, cycle) = cycle;
            self.log("Cycle:");
            for line in self.ge.format_cycle_rows(path, cycle) {
                self.log(line);
            }
            self.log("");
        }

        for path in &r.ends {
            self.log("End:");
            for line in self.ge.format_rows(path) {
                self.log(line);
            }
            self.log("");
        }

        return true;
    }

    //fn debug_enter(&self, path: &Vec<GolKeyNode<B>>) {
    //    eprintln!("Enter search {}", path.len());
    //    for line in self.ge.format_rows(path) {
    //        eprintln!("{}", line);
    //    }
    //}

    fn debug_checkpoint(&mut self, tree: &Tree<GolNode<B>>) {
        if let Some(ref output_dir) = self.output_dir {
            let path1 = format!("{}/{}", output_dir, ".tree.tmp");
            let path2 = format!("{}/{}", output_dir, "tree");
            let mut f = File::create(&path1).unwrap();
            let s = serde_json::to_string(&tree.to_serde_proxy()).unwrap();
            f.write_all(s.as_bytes()).unwrap();
            std::fs::rename(&path1, &path2).unwrap();
        }
    }

    fn debug_longest(&mut self, path: &Vec<GolKeyNode<B>>) {
        self.log(format!("Longest {}", path.len()));
        for line in self.ge.format_rows(path) {
            self.log(line);
        }
        self.log("");
    }
}
