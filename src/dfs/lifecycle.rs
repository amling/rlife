use crate::bfs;
use crate::dfs;

use bfs::bfs2::Bfs2State;
use dfs::Tree;
use dfs::graph::DfsNode;
use dfs::res::DfsRes;

pub enum LogLevel {
    INFO,
    DEBUG,
}

impl LogLevel {
    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::INFO => "INFO",
            LogLevel::DEBUG => "DEBUG",
        }
    }
}

pub trait DfsLifecycle<N: DfsNode> {
    fn threads(&self) -> usize;
    fn recollect_ms(&self) -> u64;
    fn max_mem(&self) -> usize;
    fn on_recollect_firstest(&mut self, firstest: (Vec<N::KN>, N));
    fn on_recollect_results(&mut self, r: DfsRes<N::KN>) -> bool;
    fn log(&mut self, level: LogLevel, msg: impl AsRef<str>);

    fn debug_enter(&self, _path: (&Vec<N::KN>, &N)) {
    }

    fn debug_cycle(&self, _path: &Vec<N::KN>, _cycle: &Vec<N::KN>, _last: &N::KN) {
    }

    fn debug_end(&self, _path: &Vec<N::KN>, _label: &'static str) {
    }

    fn debug_dfs_checkpoint(&mut self, _tree: &Tree<N>) {
    }

    fn debug_bfs2_checkpoint<'a>(&mut self, _get_state: impl FnOnce(&mut Self) -> &'a Bfs2State<N, N::KN>) where N: 'a {
    }

    fn debug_longest(&mut self, _path: &Vec<N::KN>) {
    }
}
