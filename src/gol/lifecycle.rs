use ars_ds::scalar::UScalar;
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
use dfs::res::DfsRes;
use gol::graph::GolDy;
use gol::graph::GolGraph;
use gol::graph::GolKeyNode;
use gol::graph::GolNode;

pub struct GolLifecycle<'a, B: UScalar> {
    pub ge: &'a GolGraph<B>,
    pub threads: usize,
    pub recollect_ms: u64,
    pub output_dir: Option<String>,
    pub log: Option<File>,
}

impl<'a, B: UScalar> GolLifecycle<'a, B> {
    fn log(&mut self, s: impl Into<String>) {
        let s = s.into();
        if let Some(log) = &mut self.log {
            writeln!(log, "{}", s).unwrap();
        }
        else {
            println!("{}", s);
        }
    }
}

impl<'a, B: UScalar + Serialize, Y: GolDy + Serialize> DfsLifecycle<GolNode<B, Y>> for GolLifecycle<'a, B> {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
    }

    fn on_recollect_firstest(&mut self, firstest: (Vec<GolKeyNode<B>>, GolNode<B, Y>)) {
        eprintln!("Recollect firstest...");
        for line in self.ge.format_rows(&firstest.0, Some(&firstest.1)) {
            eprintln!("{}", line);
        }
    }

    fn on_recollect_results(&mut self, r: DfsRes<GolKeyNode<B>>) -> bool {
        for (path, cycle, last) in &r.cycles {
            self.log("Cycle:");
            for line in self.ge.format_cycle_rows(path, cycle, last) {
                self.log(line);
            }
            self.log("");
        }

        for (path, label) in &r.ends {
            self.log(format!("End {:?}:", label));
            for line in self.ge.format_rows::<()>(path, None) {
                self.log(line);
            }
            self.log("");
        }

        return true;
    }

    //fn debug_enter(&self, path: &Vec<GolKeyNode<B>>) {
    //    eprintln!("Enter search {}", path.len());
    //    for line in self.ge.format_rows(path) {
    //        eprintln!("{}", line);
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
            let tree = tree.as_ref().map(&mut |t| t.to_serde_proxy(self.ge));
            let tree = tree.to_serde_proxy();
            serde_json::to_writer(f, &tree).unwrap();
            std::fs::rename(&path1, &path2).unwrap();
        }
    }

    fn debug_longest(&mut self, path: &Vec<GolKeyNode<B>>) {
        self.log(format!("Longest {}", path.len()));
        for line in self.ge.format_rows::<()>(path, None) {
            self.log(line);
        }
        self.log("");
    }
}
