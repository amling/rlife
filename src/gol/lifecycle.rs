use std::fs::File;
use std::io::Write;

use crate::bits;
use crate::dfs;
use crate::gol;

use bits::Bits;
use dfs::Tree;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsResVec;
use gol::graph::GolGraph;
use gol::graph::GolNode;

pub struct GolLifecycle<'a> {
    pub ge: &'a GolGraph,
    pub threads: usize,
    pub recollect_ms: u64,
    pub output_dir: Option<String>,
}

impl<'a> GolLifecycle<'a> {
    fn print_cycle<B: Bits>(&self, path: &Vec<GolNode<B>>, cycle: &Vec<GolNode<B>>) {
        println!("Cycle:");
        for line in self.ge.format_cycle_rows(path, cycle) {
            println!("{}", line);
        }
        println!("");
    }

    fn print_end<B: Bits>(&self, path: &Vec<GolNode<B>>) {
        println!("End:");
        for line in self.ge.format_rows(path) {
            println!("{}", line);
        }
        println!("");
    }
}

impl<'a, B: Bits> DfsLifecycle<GolNode<B>, DfsResVec<GolNode<B>>> for GolLifecycle<'a> {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
    }

    fn on_recollect_firstest(&self, firstest: Vec<GolNode<B>>) {
        eprintln!("Recollect firstest...");
        for line in self.ge.format_rows(&firstest) {
            eprintln!("{}", line);
        }
    }

    fn on_recollect_results(&self, r: DfsResVec<GolNode<B>>) -> bool {
        for cycle in &r.cycles {
            self.print_cycle(&cycle.0, &cycle.1);
        }

        for end in &r.ends {
            self.print_end(end);
        }

        return true;
    }

    fn debug_enter(&self, path: &Vec<GolNode<B>>) {
        //eprintln!("Enter search {}", path.len());
        //for line in self.ge.format_rows(path) {
        //    eprintln!("{}", line);
        //}
    }

    fn debug_cycle(&self, path: &Vec<GolNode<B>>, cycle: &Vec<GolNode<B>>) {
        self.print_cycle(path, cycle);
        //if path.len() + cycle.len() > 2 {
        //    panic!();
        //}
    }

    fn debug_end(&self, path: &Vec<GolNode<B>>) {
        self.print_end(path);
        //if path.len() > 2 {
        //    panic!();
        //}
    }

    fn debug_checkpoint(&self, tree: &Tree<GolNode<B>>) {
        if let Some(ref output_dir) = self.output_dir {
            let path1 = format!("{}/{}", output_dir, ".tree.tmp");
            let path2 = format!("{}/{}", output_dir, "tree");
            let mut f = File::create(&path1).unwrap();
            let s = serde_json::to_string(&tree.to_serde_proxy()).unwrap();
            f.write_all(s.as_bytes()).unwrap();
            std::fs::rename(&path1, &path2).unwrap();
        }
    }

    fn debug_longest(&self, path: &Vec<GolNode<B>>) {
        eprintln!("New longest {}", path.len());
        for line in self.ge.format_rows(path) {
            eprintln!("{}", line);
        }
    }
}
