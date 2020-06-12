use serde::Serialize;

use crate::gol;
use crate::lgol;

use gol::lifecycle::GolGraphTrait;
use lgol::graph::LGolAxis;
use lgol::graph::LGolBgCoord;
use lgol::graph::LGolGraph;
use lgol::graph::LGolKeyNode;
use lgol::graph::LGolNode;
use lgol::graph::RowTuple;

impl<BS: RowTuple + Serialize, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>> GolGraphTrait for LGolGraph<BS, BC, UA, VA> where BS::Item: Serialize, BC::V: Serialize, UA::S: Serialize, VA::S: Serialize {
    type N = LGolNode<BS, BC::V, UA::S, VA::S>;
    type FN = LGolNode<BS, BC::V, UA::S, VA::S>;

    fn format_rows(&self, rows: &Vec<LGolKeyNode<BS>>, last: Option<&LGolNode<BS, BC::V, UA::S, VA::S>>) -> Vec<String> {
        self.format_rows(rows, last)
    }

    fn format_cycle_rows(&self, path: &Vec<LGolKeyNode<BS>>, cycle: &Vec<LGolKeyNode<BS>>, last: &LGolKeyNode<BS>) -> Vec<String> {
        self.format_cycle_rows(path, cycle, last)
    }

    fn format_cycle_shape(&self, path: &Vec<LGolKeyNode<BS>>, cycle: &Vec<LGolKeyNode<BS>>, _last: &LGolKeyNode<BS>) -> String {
        // we could do better...
        format!("init {} cycle {}", path.len(), cycle.len())
    }

    fn freeze_dfs_node(&self, n: &LGolNode<BS, BC::V, UA::S, VA::S>) -> LGolNode<BS, BC::V, UA::S, VA::S> {
        n.clone()
    }
}
