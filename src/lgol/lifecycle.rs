use chrono::Local;
use serde::Serialize;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::dfs;
use crate::gol;
use crate::lgol;
use crate::sal;

use dfs::Tree;
use dfs::lifecycle::DfsLifecycle;
use dfs::lifecycle::LogLevel;
use dfs::res::DfsRes;
use gol::lifecycle::GolRctlEp;
use lgol::graph::LGolGraph;
use lgol::graph::LGolKeyNode;
use lgol::graph::LGolNode;
use lgol::graph::RowTuple;
use sal::SerdeFormat;

pub struct LGolLifecycle<'a, BS: RowTuple> {
    pub ge: &'a LGolGraph<BS>,
    pub ep: Arc<GolRctlEp>,
}

impl<'a, BS: RowTuple + Serialize> DfsLifecycle<LGolNode<BS>> for LGolLifecycle<'a, BS> where BS::Item: Serialize {
    fn threads(&self) -> usize {
        self.ep.threads.load(Ordering::Relaxed)
    }

    fn recollect_ms(&self) -> u64 {
        self.ep.recollect_ms.load(Ordering::Relaxed)
    }

    fn max_mem(&self) -> usize {
        self.ep.max_mem.load(Ordering::Relaxed)
    }

    fn on_recollect_firstest(&mut self, firstest: (Vec<LGolKeyNode<BS>>, LGolNode<BS>)) {
        self.log(LogLevel::DEBUG, "Recollect firstest...");
        for line in self.ge.format_rows(&firstest.0, Some(&firstest.1)) {
            self.log(LogLevel::DEBUG, line);
        }
    }

    fn on_recollect_results(&mut self, r: DfsRes<LGolKeyNode<BS>>) -> bool {
        for (path, cycle, last) in &r.cycles {
            self.log(LogLevel::INFO, "Cycle:");
            for line in self.ge.format_cycle_rows(path, cycle, last) {
                self.log(LogLevel::INFO, line);
            }
            self.log(LogLevel::INFO, "");
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

    fn debug_dfs_checkpoint(&mut self, tree: &Tree<LGolNode<BS>>) {
        self.ep.checkpt_rq.service(&mut |path, mut log| {
            let path = match path {
                Some(path) => path,
                None => Local::now().format("tree.%Y%m%d-%H%M%S.json").to_string(),
            };

            let t0 = std::time::Instant::now();
            let tree = tree.to_serde_proxy();
            SerdeFormat::JSON.write(&path, &tree).unwrap();

            log.log(format!("Checkpointed DFS state to {} in {:?}", path, t0.elapsed()));
        });
    }

    fn debug_longest(&mut self, path: &Vec<LGolKeyNode<BS>>) {
        self.log(LogLevel::INFO, format!("Longest {}", path.len()));
        for line in self.ge.format_rows(path, None) {
            self.log(LogLevel::INFO, line);
        }
        self.log(LogLevel::INFO, "");
    }
}
