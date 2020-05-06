use ars_ds::scalar::UScalar;
use chrono::Local;
use serde::Serialize;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::time::Duration;
use std::time::SystemTime;

use crate::dfs;
use crate::gol;

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

pub struct GolLifecycle<'a, B: UScalar, Y: GolDy, F: GolForce<Y>, E: GolEnds<B>> {
    pub ge: &'a GolGraph<B, Y, F, E>,
    pub threads: usize,
    pub recollect_ms: u64,
    pub output_dir: Option<String>,
    pub log: Option<File>,
}

impl<'a, B: UScalar + Serialize, Y: GolDy + Serialize, F: GolForce<Y>, E: GolEnds<B>> DfsLifecycle<GolNode<B, Y>> for GolLifecycle<'a, B, Y, F, E> {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
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
        }
    }

    //fn debug_enter(&self, path: &Vec<GolKeyNode<B>>) {
    //    self.log(LogLevel::INFO, format!("Enter search {}", path.len()));
    //    for line in self.ge.format_rows(path) {
    //        self.log(LogLevel::INFO, line);
    //    }
    //}

    fn debug_checkpoint(&mut self, tree: &Tree<GolNode<B, Y>>) {
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
            let f = File::create(&path1).unwrap();
            let f = BufWriter::new(f);
            let tree = tree.as_ref().map(&mut |n| self.ge.params.freeze_node(n));
            let tree = tree.to_serde_proxy();
            serde_json::to_writer(f, &tree).unwrap();
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
