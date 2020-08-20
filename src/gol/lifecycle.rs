use ars_ds::scalar::UScalar;
use ars_rctl_core::RctlLog;
use ars_rctl_derive::rctl_ep;
use ars_rctl_main::rq::RctlDeferredWrite;
use ars_rctl_main::rq::RctlRunQueue;
use chrono::Local;
use serde::Serialize;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use crate::bfs;
use crate::dfs;
use crate::gol;
use crate::sal;

use bfs::bfs2::Bfs2ChunkFactory;
use bfs::bfs2::Bfs2Dedupe;
use bfs::bfs2::Bfs2State;
use dfs::Tree;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use dfs::res::DfsRes;
use gol::ends::GolEnds;
use gol::graph::GolDy;
use gol::graph::GolForce;
use gol::graph::GolGraph;
use gol::graph::GolKeyNode;
use gol::graph::GolNode;
use gol::graph::GolNodeSerdeProxy;
use sal::JsonSerializer;
use sal::SerializerFor;

pub struct GolRctlEp {
    pub threads: AtomicUsize,
    pub recollect_ms: AtomicU64,
    pub max_mem: AtomicUsize,
    pub checkpt_rq: RctlRunQueue<(Option<String>, RctlDeferredWrite<()>)>,
}

#[rctl_ep]
impl GolRctlEp {
    fn set_threads(&self, threads: usize) {
        self.threads.store(threads, Ordering::Relaxed);
    }

    fn get_threads(&self) -> usize {
        self.threads.load(Ordering::Relaxed)
    }

    fn set_recollect_ms(&self, recollect_ms: u64) {
        self.recollect_ms.store(recollect_ms, Ordering::Relaxed);
    }

    fn get_recollect_ms(&self) -> u64 {
        self.recollect_ms.load(Ordering::Relaxed)
    }

    fn set_max_mem(&self, max_mem: usize) {
        self.max_mem.store(max_mem, Ordering::Relaxed);
    }

    fn set_max_mem_mb(&self, max_mem_mb: usize) {
        self.max_mem.store(max_mem_mb << 20, Ordering::Relaxed);
    }

    fn set_max_mem_gb(&self, max_mem_gb: usize) {
        self.max_mem.store(max_mem_gb << 30, Ordering::Relaxed);
    }

    fn get_max_mem(&self) -> usize {
        self.max_mem.load(Ordering::Relaxed)
    }

    fn checkpt(&self, log: RctlLog) {
        let (r, w) = ars_rctl_main::rq::deferred();
        self.checkpt_rq.push((None, w));
        r.wait(log)
    }

    fn checkpt_to(&self, path: String, log: RctlLog) {
        let (r, w) = ars_rctl_main::rq::deferred();
        self.checkpt_rq.push((Some(path), w));
        r.wait(log)
    }
}

pub trait GolGraphTrait {
    type N: DfsNode + Serialize;
    type FN: Clone + Serialize;

    fn format_rows(&self, rows: &Vec<<Self::N as DfsNode>::KN>, last: Option<&Self::N>) -> Vec<String>;
    fn format_cycle_rows(&self, path: &Vec<<Self::N as DfsNode>::KN>, cycle: &Vec<<Self::N as DfsNode>::KN>, last: &<Self::N as DfsNode>::KN) -> Vec<String>;
    fn format_cycle_rows_hack(&self, cycle: &Vec<<Self::N as DfsNode>::KN>) -> Option<Vec<String>>;
    fn format_cycle_shape(&self, path: &Vec<<Self::N as DfsNode>::KN>, cycle: &Vec<<Self::N as DfsNode>::KN>, last: &<Self::N as DfsNode>::KN) -> String;
    fn freeze_dfs_node(&self, n: &Self::N) -> Self::FN;
}

impl<B: UScalar + Serialize, Y: GolDy + Serialize, F: GolForce<Y>, E: GolEnds<B>> GolGraphTrait for GolGraph<B, Y, F, E> {
    type N = GolNode<B, Y>;
    type FN = GolNodeSerdeProxy<B, Y>;

    fn format_rows(&self, rows: &Vec<GolKeyNode<B>>, last: Option<&GolNode<B, Y>>) -> Vec<String> {
        self.params.format_rows(rows, last)
    }

    fn format_cycle_rows(&self, path: &Vec<GolKeyNode<B>>, cycle: &Vec<GolKeyNode<B>>, last: &GolKeyNode<B>) -> Vec<String> {
        self.params.format_cycle_rows(path, cycle, last)
    }

    fn format_cycle_rows_hack(&self, _cycle: &Vec<GolKeyNode<B>>) -> Option<Vec<String>> {
        None
    }

    fn format_cycle_shape(&self, path: &Vec<GolKeyNode<B>>, cycle: &Vec<GolKeyNode<B>>, last: &GolKeyNode<B>) -> String {
        format!("init {} dx {} dy {}", path.len(), last.dx - cycle[0].dx, cycle.len())
    }

    fn freeze_dfs_node(&self, n: &GolNode<B, Y>) -> GolNodeSerdeProxy<B, Y> {
        self.params.freeze_node(n)
    }
}

pub struct GolLifecycle<'a, GE> {
    pub ge: &'a GE,
    pub ep: Arc<GolRctlEp>,
}

impl<'a, GE: GolGraphTrait> DfsLifecycle<GE::N> for GolLifecycle<'a, GE> where <GE::N as DfsNode>::KN: Serialize, <<GE::N as DfsNode>::KN as DfsKeyNode>::HN: Serialize {
    fn threads(&self) -> usize {
        self.ep.threads.load(Ordering::Relaxed)
    }

    fn recollect_ms(&self) -> u64 {
        self.ep.recollect_ms.load(Ordering::Relaxed)
    }

    fn max_mem(&self) -> usize {
        self.ep.max_mem.load(Ordering::Relaxed)
    }

    fn on_recollect_firstest(&mut self, firstest: (Vec<<GE::N as DfsNode>::KN>, GE::N)) {
        self.log(LogLevel::DEBUG, "Recollect firstest...");
        for line in self.ge.format_rows(&firstest.0, Some(&firstest.1)) {
            self.log(LogLevel::DEBUG, line);
        }
    }

    fn on_recollect_results(&mut self, r: DfsRes<<GE::N as DfsNode>::KN>) -> bool {
        for (path, cycle, last) in &r.cycles {
            let shape = self.ge.format_cycle_shape(path, cycle, last);
            self.log(LogLevel::INFO, format!("Cycle ({}):", shape));
            for line in self.ge.format_cycle_rows(path, cycle, last) {
                self.log(LogLevel::INFO, line);
            }
            self.log(LogLevel::INFO, "");
            if let Some(lines) = self.ge.format_cycle_rows_hack(cycle) {
                self.log(LogLevel::INFO, format!("Cycle rows ({}):", shape));
                for line in lines {
                    self.log(LogLevel::INFO, line);
                }
                self.log(LogLevel::INFO, "");
            }
        }

        for (path, label) in &r.ends {
            self.log(LogLevel::INFO, format!("End {:?}:", label));
            for line in self.ge.format_rows(path, None) {
                self.log(LogLevel::INFO, line);
            }
            self.log(LogLevel::INFO, "");
        }

        return true;
    }

    fn log(&self, level: LogLevel, msg: impl AsRef<str>) {
        let msg = msg.as_ref();
        let msg = format!("{} [{}] {}", Local::now().format("%Y%m%d %H:%M:%S"), level.name(), msg);
        println!("{}", msg);
        std::io::stdout().flush().unwrap();
    }

    //fn debug_enter(&self, path: &Vec<<GE::N as DfsNode>::KN>) {
    //    self.log(LogLevel::INFO, format!("Enter search {}", path.len()));
    //    for line in self.ge.format_rows(path) {
    //        self.log(LogLevel::INFO, line);
    //    }
    //}

    fn debug_dfs_checkpoint(&mut self, tree: &Tree<GE::N>) {
        self.ep.checkpt_rq.service(&mut |(path, mut w)| {
            let path = match path {
                Some(path) => path,
                None => Local::now().format("tree.%Y%m%d-%H%M%S.json").to_string(),
            };

            let t0 = std::time::Instant::now();
            let tree = tree.as_ref().map(&mut |n| self.ge.freeze_dfs_node(n));
            let tree = tree.to_serde_proxy();
            JsonSerializer().to_file(&path, &tree).unwrap();

            let msg = format!("Checkpointed DFS state to {} in {:?}", path, t0.elapsed());
            w.output(&msg);
            self.log(LogLevel::INFO, &msg);

            w.ret(())
        });
    }

    fn debug_bfs2_checkpoint<'b, CF: Bfs2ChunkFactory<GE::N> + 'b, D: Bfs2Dedupe<GE::N, CF> + 'b>(&mut self, get_state: impl FnOnce(&mut Self) -> &'b Bfs2State<GE::N, CF, D>) where GE::N: 'b {
        // clone ep so self is still available for closure to take
        let ep = self.ep.clone();

        let mut maybe_state = None;
        let mut maybe_get_state = Some(get_state);

        ep.checkpt_rq.service(&mut |(path, mut w)| {
            let path = match path {
                Some(path) => path,
                None => Local::now().format("bfs2.%Y%m%d-%H%M%S.bin").to_string(),
            };

            let t0 = std::time::Instant::now();

            // Arggh, this is very stupid, but Either<state, get_state> doesn't really work out
            // (still have to "take" it to call it, etc.).
            let state = maybe_state.get_or_insert_with(|| {
                // no state yet, better still have the getter
                let get_state = maybe_get_state.take().unwrap();
                get_state(self)
            });

            state.serializer().to_file(&path, state).unwrap();

            let msg = format!("Checkpointed BFS state to {} in {:?}", path, t0.elapsed());
            w.output(&msg);
            self.log(LogLevel::INFO, &msg);

            w.ret(())
        });
    }

    fn debug_longest(&mut self, path: &Vec<<GE::N as DfsNode>::KN>) {
        self.log(LogLevel::INFO, format!("Longest {}", path.len()));
        for line in self.ge.format_rows(path, None) {
            self.log(LogLevel::INFO, line);
        }
        self.log(LogLevel::INFO, "");
    }
}
