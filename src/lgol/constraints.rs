#![allow(unused_parens)]

use ars_ds::nice::Nice;

use crate::lgol;

use lgol::axis::LGolAxis;
use lgol::bg::LGolBgCoord;
use lgol::ends::LGolEnds;
use lgol::graph::LGolGraph;
use lgol::graph::RowTuple;

pub trait LGolConstraint<BC: LGolBgCoord>: Copy {
    type S: Nice;

    fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Self::S;
    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, s0: Self::S, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Self::S>;
}

impl<BC: LGolBgCoord> LGolConstraint<BC> for () {
    type S = ();

    fn zero_stat<BS: RowTuple>(&self, _ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) {
    }

    fn add_stat<BS: RowTuple>(&self, _ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, _s0: (), _bg_coord: BC, _r: BS::Item, _idx: usize, _v: bool) -> Option<()> {
        Some(())
    }
}

impl<BC: LGolBgCoord, CS1: LGolConstraint<BC>, CS2: LGolConstraint<BC>> LGolConstraint<BC> for (CS1, CS2) {
    type S = (CS1::S, CS2::S);

    fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Self::S {
        let (ref cs1, ref cs2) = self;
        (cs1.zero_stat(ge), cs2.zero_stat(ge))
    }

    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, (s1, s2): Self::S, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Self::S> {
        let (ref cs1, ref cs2) = self;
        Some((
            match cs1.add_stat(ge, s1, bg_coord, r, idx, v) {
                Some(s) => s,
                None => {
                    return None;
                },
            },
            match cs2.add_stat(ge, s2, bg_coord, r, idx, v) {
                Some(s) => s,
                None => {
                    return None;
                },
            },
        ))
    }
}
