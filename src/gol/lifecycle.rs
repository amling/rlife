use ars_ds::scalar::UScalar;
use ars_rctl_core::RctlLog;
use ars_rctl_derive::rctl_ep;
use ars_rctl_main::rq::RctlRunQueue;
use chrono::Local;
use serde::Serialize;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::bfs;
use crate::dfs;
use crate::gol;
use crate::sal;

use bfs::bfs2::Bfs2State;
use dfs::Tree;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use dfs::res::DfsRes;
use gol::ends::GolEnds;
use gol::graph::GolDy;
use gol::graph::GolForce;
use gol::graph::GolGraph;
use gol::graph::GolKeyNode;
use gol::graph::GolNode;
use sal::SerdeFormat;

pub struct GolRctlEp {
    pub threads: usize,
    pub recollect_ms: u64,
    pub max_mem: AtomicUsize,
    pub checkpt_rq: RctlRunQueue<Option<String>, ()>,
}

#[rctl_ep]
impl GolRctlEp {
    fn set_max_mem(&self, max_mem: usize) {
        self.max_mem.store(max_mem, Ordering::Relaxed);
    }

    fn get_max_mem(&self) -> usize {
        self.max_mem.load(Ordering::Relaxed)
    }

    fn checkpt(&self, log: RctlLog) {
        self.checkpt_rq.run(None, log);
    }

    fn checkpt_to(&self, path: String, log: RctlLog) {
        self.checkpt_rq.run(Some(path), log);
    }
}

pub struct GolLifecycle<'a, B: UScalar, Y: GolDy, F: GolForce<Y>, E: GolEnds<B>> {
    pub ge: &'a GolGraph<B, Y, F, E>,
    pub ep: Arc<GolRctlEp>,
}

impl<'a, B: UScalar + Serialize, Y: GolDy + Serialize, F: GolForce<Y>, E: GolEnds<B>> DfsLifecycle<GolNode<B, Y>> for GolLifecycle<'a, B, Y, F, E> {
    fn threads(&self) -> usize {
        self.ep.threads
    }

    fn recollect_ms(&self) -> u64 {
        self.ep.recollect_ms
    }

    fn max_mem(&self) -> usize {
        self.ep.max_mem.load(Ordering::Relaxed)
    }

    fn on_recollect_firstest(&mut self, firstest: (Vec<GolKeyNode<B>>, GolNode<B, Y>)) {
        self.log(LogLevel::DEBUG, "Recollect firstest...");
        for line in self.ge.params.format_rows(&firstest.0, Some(&firstest.1)) {
            self.log(LogLevel::DEBUG, line);
        }
    }

    fn on_recollect_results(&mut self, r: DfsRes<GolKeyNode<B>>) -> bool {
        for (path, cycle, last) in &r.cycles {
            self.log(LogLevel::INFO, "Cycle:");
            for line in self.ge.params.format_cycle_rows(path, cycle, last) {
                self.log(LogLevel::INFO, line);
            }
            self.log(LogLevel::INFO, "");
        }

        for (path, label) in &r.ends {
            self.log(LogLevel::INFO, format!("End {:?}:", label));
            for line in self.ge.params.format_rows::<B, Y>(path, None) {
                self.log(LogLevel::INFO, line);
            }
            self.log(LogLevel::INFO, "");
        }

        return true;
    }

    fn log(&mut self, level: LogLevel, msg: impl AsRef<str>) {
        let msg = msg.as_ref();
        let msg = format!("{} [{}] {}", Local::now().format("%Y%m%d %H:%M:%S"), level.name(), msg);
        println!("{}", msg);
        std::io::stdout().flush().unwrap();
    }

    //fn debug_enter(&self, path: &Vec<GolKeyNode<B>>) {
    //    self.log(LogLevel::INFO, format!("Enter search {}", path.len()));
    //    for line in self.ge.format_rows(path) {
    //        self.log(LogLevel::INFO, line);
    //    }
    //}

    fn debug_dfs_checkpoint(&mut self, tree: &Tree<GolNode<B, Y>>) {
        self.ep.checkpt_rq.service(&mut |path, mut log| {
            let path = match path {
                Some(path) => path,
                None => Local::now().format("tree.%Y%m%d-%H%M%S.json").to_string(),
            };

            let t0 = std::time::Instant::now();
            let tree = tree.as_ref().map(&mut |n| self.ge.params.freeze_node(n));
            let tree = tree.to_serde_proxy();
            SerdeFormat::JSON.write(&path, &tree).unwrap();

            log.log(format!("Checkpointed DFS state to {} in {:?}", path, t0.elapsed()));
        });
    }

    fn debug_bfs2_checkpoint<'b>(&mut self, get_state: impl FnOnce(&mut Self) -> &'b Bfs2State<GolNode<B, Y>, GolKeyNode<B>>) where GolNode<B, Y>: 'b {
        // clone ep so self is still available for closure to take
        let ep = self.ep.clone();

        let mut maybe_state = None;
        let mut maybe_get_state = Some(|| get_state(self));

        ep.checkpt_rq.service(&mut |path, mut log| {
            let path = match path {
                Some(path) => path,
                None => Local::now().format("bfs2.%Y%m%d-%H%M%S.bin").to_string(),
            };

            let t0 = std::time::Instant::now();

            // Arggh, this is very stupid, but Either<state, get_state> doesn't really work out
            // (still have to "take" it to call it, etc.).
            let state = maybe_state.get_or_insert_with(|| {
                // no state yet, better still have the getter
                maybe_get_state.take().unwrap()()
            });

            SerdeFormat::Bincode.write(&path, state).unwrap();

            log.log(format!("Checkpointed BFS state to {} in {:?}", path, t0.elapsed()));
        });
    }

    fn debug_longest(&mut self, path: &Vec<GolKeyNode<B>>) {
        self.log(LogLevel::INFO, format!("Longest {}", path.len()));
        for line in self.ge.params.format_rows::<B, Y>(path, None) {
            self.log(LogLevel::INFO, line);
        }
        self.log(LogLevel::INFO, "");
    }
}
