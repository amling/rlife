use ars_aa::lattice::LatticeCanonicalizable;
use ars_aa::lattice::LatticeCanonicalizer;
use serde::Serialize;

use crate::gol;
use crate::lgol;

use gol::lifecycle::GolGraphTrait;
use lgol::axis::LGolAxis;
use lgol::bg::LGolBgCoord;
use lgol::ends::LGolEnds;
use lgol::graph::LGolGraph;
use lgol::graph::LGolKeyNode;
use lgol::graph::LGolNode;
use lgol::graph::RowTuple;
use lgol::lat1::Vec3;

impl<BS: RowTuple + Serialize, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, E: LGolEnds<BS>> GolGraphTrait for LGolGraph<BS, BC, UA, VA, E> where BS::Item: Serialize, BC: Serialize, UA::S: Serialize, VA::S: Serialize {
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
            (du, dv, dw)
        };
        let dcycle = {
            let du = (last.du - cycle[0].du) as isize;
            let dv = (last.dv - cycle[0].dv) as isize;
            let dw = (cycle.len() as isize) * self.lat1.adet;
            (du, dv, dw)
        };

        let dpath = self.lat1.uvw_to_xyt(dpath);
        let dcycle = self.lat1.uvw_to_xyt(dcycle);

        let mut wraps = vec![];
        if self.params.u_axis.wrap_in_print() {
            wraps.push(self.params.vu);
        }
        if self.params.v_axis.wrap_in_print() {
            wraps.push(self.params.vv);
        }
        let wraps = Vec3::canonicalize(wraps);

        let dpath = wraps.canonicalize(dpath);
        let dcycle = wraps.canonicalize(dcycle);

        format!("path delta {:?} cycle delta {:?}", dpath, dcycle)
    }

    fn freeze_dfs_node(&self, n: &LGolNode<BS, BC, UA::S, VA::S>) -> LGolNode<BS, BC, UA::S, VA::S> {
        n.clone()
    }
}
