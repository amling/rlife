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
            println!("Cycle:");
            self.ge.print_rows(&cycle.0);
            self.ge.print_dash_row();
            self.ge.print_rows(&cycle.1);
            println!("");
        }

        for end in &r.ends {
            println!("End:");
            self.ge.print_rows(end);
            println!("");
        }

        return true;
    }

    fn debug_enter(&self, path: &Vec<(B, B)>) {
        println!("Enter search");
        self.ge.print_rows(path);
    }
}
