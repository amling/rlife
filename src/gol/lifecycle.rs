use crate::bits;
use crate::dfs;
use crate::gol;

use bits::Bits;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsResVec;
use gol::graph::GolGraph;
use gol::graph::GolNode;

pub struct GolLifecycle<'a> {
    pub ge: &'a GolGraph,
    pub threads: usize,
    pub recollect_ms: u64,
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

    fn on_recollect(&self, firstest: Vec<GolNode<B>>, r: DfsResVec<GolNode<B>>) -> bool {
        eprintln!("Recollect...");

        eprintln!("Firstest");
        for line in self.ge.format_rows(&firstest) {
            eprintln!("{}", line);
        }

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
}
