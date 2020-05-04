use crate::dfs;

use dfs::Tree;
use dfs::graph::DfsNode;
use dfs::res::DfsRes;

pub trait DfsLifecycle<N: DfsNode> {
    fn threads(&self) -> usize;
    fn recollect_ms(&self) -> u64;
    fn on_recollect_firstest(&mut self, firstest: (Vec<N::KN>, N));
    fn on_recollect_results(&mut self, r: DfsRes<N::KN>) -> bool;

    fn debug_enter(&self, _path: (&Vec<N::KN>, &N)) {
    }

    fn debug_cycle(&self, _path: &Vec<N::KN>, _cycle: &Vec<N::KN>, _last: &N::KN) {
    }

    fn debug_end(&self, _path: &Vec<N::KN>, _label: &'static str) {
    }

    fn debug_checkpoint(&mut self, _tree: &Tree<N>) {
    }

    fn debug_longest(&mut self, _path: &Vec<N::KN>) {
    }
}
