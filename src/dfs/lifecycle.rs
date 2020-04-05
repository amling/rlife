use crate::dfs;

use dfs::Tree;

pub trait DfsLifecycle<N, KN, R> {
    fn threads(&self) -> usize;
    fn recollect_ms(&self) -> u64;
    fn on_recollect_firstest(&mut self, firstest: Vec<KN>);
    fn on_recollect_results(&mut self, r: R) -> bool;

    fn debug_enter(&self, _path: &Vec<KN>) {
    }

    fn debug_cycle(&self, _path: &Vec<KN>, _cycle: &Vec<KN>) {
    }

    fn debug_end(&self, _path: &Vec<KN>) {
    }

    fn debug_checkpoint(&mut self, _tree: &Tree<N>) {
    }

    fn debug_longest(&mut self, _path: &Vec<KN>) {
    }
}
