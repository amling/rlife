use serde::Serialize;

use crate::gol;
use crate::lgol;

use gol::lifecycle::GolGraphTrait;
use lgol::axis::LGolAxis;
use lgol::bg::LGolBgCoord;
use lgol::graph::LGolGraph;
use lgol::graph::LGolKeyNode;
use lgol::graph::LGolNode;
use lgol::graph::RowTuple;

impl<BS: RowTuple + Serialize, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>> GolGraphTrait for LGolGraph<BS, BC, UA, VA> where BS::Item: Serialize, BC: Serialize, UA::S: Serialize, VA::S: Serialize {
    type N = LGolNode<BS, BC, UA::S, VA::S>;
    type FN = LGolNode<BS, BC, UA::S, VA::S>;

    fn format_rows(&self, rows: &Vec<LGolKeyNode<BS>>, last: Option<&LGolNode<BS, BC, UA::S, VA::S>>) -> Vec<String> {
        self.format_rows(rows, last)
    }

    fn format_cycle_rows(&self, path: &Vec<LGolKeyNode<BS>>, cycle: &Vec<LGolKeyNode<BS>>, last: &LGolKeyNode<BS>) -> Vec<String> {
        self.format_cycle_rows(path, cycle, last)
    }

    fn format_cycle_shape(&self, path: &Vec<LGolKeyNode<BS>>, cycle: &Vec<LGolKeyNode<BS>>, last: &LGolKeyNode<BS>) -> String {
        let dpath = {
            let du = cycle[0].du as isize;
            let dv = cycle[0].dv as isize;
            let dw = (path.len() as isize) * self.lat1.adet;
            self.lat1.uvw_to_xyt((du, dv, dw))
        };
        let dcycle = {
            let du = (last.du - cycle[0].du) as isize;
            let dv = (last.dv - cycle[0].dv) as isize;
            let dw = (cycle.len() as isize) * self.lat1.adet;
            self.lat1.uvw_to_xyt((du, dv, dw))
        };
        format!("path delta {:?} cycle delta {:?}", dpath, dcycle)
    }

    fn freeze_dfs_node(&self, n: &LGolNode<BS, BC, UA::S, VA::S>) -> LGolNode<BS, BC, UA::S, VA::S> {
        n.clone()
    }
}
