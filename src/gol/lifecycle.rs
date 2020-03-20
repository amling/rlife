use crate::bits;
use crate::dfs;
use crate::gol;

use bits::Bits;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsResVec;
use gol::graph::GolGraph;

pub struct GolLifecycle<'a> {
    pub ge: &'a GolGraph,
    pub threads: usize,
    pub recollect_ms: u64,
}

impl<'a> GolLifecycle<'a> {
    fn print_cycle<B: Bits>(&self, path: &Vec<(B, B)>, cycle: &Vec<(B, B)>) {
        println!("Cycle:");
        self.ge.print_rows(&path);
        self.ge.print_dash_row();
        self.ge.print_rows(&cycle);
        println!("");
    }

    fn print_end<B: Bits>(&self, path: &Vec<(B, B)>) {
        println!("End:");
        self.ge.print_rows(path);
        println!("");
    }
}

impl<'a, B: Bits> DfsLifecycle<(B, B), DfsResVec<(B, B)>> for GolLifecycle<'a> {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
    }

    fn on_recollect(&self, r: DfsResVec<(B, B)>) -> bool {
        println!("Recollect...");

        // TODO: actual status

        for cycle in &r.cycles {
            self.print_cycle(&cycle.0, &cycle.1);
        }

        for end in &r.ends {
            self.print_end(end);
        }

        return true;
    }

    fn debug_enter(&self, path: &Vec<(B, B)>) {
        //println!("Enter search");
        //self.ge.print_rows(path);
    }

    fn debug_cycle(&self, path: &Vec<(B, B)>, cycle: &Vec<(B, B)>) {
        //self.print_cycle(path, cycle);
    }

    fn debug_end(&self, path: &Vec<(B, B)>) {
        //self.print_end(path);
    }
}
