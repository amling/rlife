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
    fn print_cycle<B: Bits>(&self, path: &Vec<(B, B, B, usize)>, cycle: &Vec<(B, B, B, usize)>) {
        println!("Cycle:");
        self.ge.print_rows(&path);
        self.ge.print_dash_row();
        self.ge.print_rows(&cycle);
        println!("");
    }

    fn print_end<B: Bits>(&self, path: &Vec<(B, B, B, usize)>) {
        println!("End:");
        self.ge.print_rows(path);
        println!("");
    }
}

impl<'a, B: Bits> DfsLifecycle<(B, B, B, usize), DfsResVec<(B, B, B, usize)>> for GolLifecycle<'a> {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
    }

    fn on_recollect(&self, deepest: Vec<(B, B, B, usize)>, r: DfsResVec<(B, B, B, usize)>) -> bool {
        println!("Recollect...");

        println!("Deepest");
        self.ge.print_rows(&deepest);

        for cycle in &r.cycles {
            self.print_cycle(&cycle.0, &cycle.1);
        }

        for end in &r.ends {
            self.print_end(end);
        }

        return true;
    }

    fn debug_enter(&self, path: &Vec<(B, B, B, usize)>) {
        //println!("Enter search {}", path.len());
        //self.ge.print_rows(path);
    }

    fn debug_cycle(&self, path: &Vec<(B, B, B, usize)>, cycle: &Vec<(B, B, B, usize)>) {
        self.print_cycle(path, cycle);
        if path.len() + cycle.len() > 2 {
            panic!();
        }
    }

    fn debug_end(&self, path: &Vec<(B, B, B, usize)>) {
        self.print_end(path);
        if path.len() > 2 {
            panic!();
        }
    }
}
