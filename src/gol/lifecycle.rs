use ars_ds::scalar::UScalar;
use ars_rctl_derive::rctl_ep;
use chrono::Local;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;

use crate::dfs;
use crate::gol;
use crate::sal;

use dfs::Tree;
use dfs::TreeStatus;
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
}

#[rctl_ep]
impl GolRctlEp {
    fn set_max_mem(&self, max_mem: usize) {
        self.max_mem.store(max_mem, Ordering::Relaxed);
    }

    fn get_max_mem(&self) -> usize {
        self.max_mem.load(Ordering::Relaxed)
    }
}

pub struct GolLifecycle<'a, B: UScalar, Y: GolDy, F: GolForce<Y>, E: GolEnds<B>> {
    pub ge: &'a GolGraph<B, Y, F, E>,
    pub ep: Arc<GolRctlEp>,
    pub output_dir: Option<String>,
    pub log: Option<File>,
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
        if let Some(log) = &mut self.log {
            writeln!(log, "{}", msg).unwrap();
        }
        else {
            println!("{}", msg);
            std::io::stdout().flush().unwrap();
        }
    }

    //fn debug_enter(&self, path: &Vec<GolKeyNode<B>>) {
    //    self.log(LogLevel::INFO, format!("Enter search {}", path.len()));
    //    for line in self.ge.format_rows(path) {
    //        self.log(LogLevel::INFO, line);
    //    }
    //}

    fn debug_dfs_checkpoint(&mut self, tree: &Tree<GolNode<B, Y>>) {
        if let Some(ref output_dir) = self.output_dir {
            let path2 = format!("{}/{}", output_dir, "tree");

            let b = (|| {
                if let Tree(_, TreeStatus::Closed) = tree {
                    return true;
                }

                // Decide if we should be doing this (it's expensive and no need to checkpoint every
                // time it's offered.
                match std::fs::metadata(&path2) {
                    Err(_) => true,
                    Ok(m) => match m.modified() {
                        Err(_) => true,
                        Ok(t) => SystemTime::now() >= t + Duration::from_secs(60),
                    },
                }
            })();
            if !b {
                return;
            }

            let path1 = format!("{}/{}", output_dir, ".tree.tmp");
            let tree = tree.as_ref().map(&mut |n| self.ge.params.freeze_node(n));
            let tree = tree.to_serde_proxy();
            SerdeFormat::JSON.write(&path1, &tree).unwrap();
            std::fs::rename(&path1, &path2).unwrap();
        }
    }

    fn debug_longest(&mut self, path: &Vec<GolKeyNode<B>>) {
        self.log(LogLevel::INFO, format!("Longest {}", path.len()));
        for line in self.ge.params.format_rows::<B, Y>(path, None) {
            self.log(LogLevel::INFO, line);
        }
        self.log(LogLevel::INFO, "");
    }
}
