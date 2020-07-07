#![allow(unused_parens)]

use ars_ds::nice::Nice;

use crate::lgol;

use lgol::axis::LGolAxis;
use lgol::bg::LGolBg;
use lgol::bg::LGolBgCoord;
use lgol::ends::LGolEnds;
use lgol::graph::LGolGraph;
use lgol::graph::RowTuple;

pub trait LGolConstraint<BC: LGolBgCoord>: Copy {
    type S: Nice;

    fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Self::S;
    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, s0: Self::S, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Self::S>;
}

macro_rules! impl_cons {
    ($([$cs_id:ident / $s_id:ident / $ty:ident])*) => {
        impl<BC: LGolBgCoord $(, $ty: LGolConstraint<BC>)*> LGolConstraint<BC> for ($($ty,)*) {
            type S = ($($ty::S,)*);

            #[allow(unused_variables)]
            fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Self::S {
                let ($(ref $cs_id,)*) = self;
                ($($cs_id.zero_stat(ge),)*)
            }

            #[allow(unused_variables)]
            fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, ($($s_id,)*): Self::S, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Self::S> {
                let ($(ref $cs_id,)*) = self;
                Some((
                    $(
                        match $cs_id.add_stat(ge, $s_id, bg_coord, r, idx, v) {
                            Some(s) => s,
                            None => {
                                return None;
                            },
                        },
                    )*
                ))
            }
        }
    }
}

impl_cons!();
impl_cons!([cs1 / s1 / CS1]);
impl_cons!([cs1 / s1 / CS1][cs2 / s2 / CS2]);
impl_cons!([cs1 / s1 / CS1][cs2 / s2 / CS2][cs3 / s3 / CS3]);

#[derive(Clone)]
#[derive(Copy)]
struct LGolConstraintUWindow<LBG, RBG> {
    // (numerator, denominator), a value of 1 is the entire width
    pub w: (isize, isize),
    pub left_bg: LBG,
    pub right_bg: RBG,
}

impl<BC: LGolBgCoord, LBG: LGolBg<BC>, RBG: LGolBg<BC>> LGolConstraint<BC> for LGolConstraintUWindow<LBG, RBG> {
    type S = (i8, i8);

    fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> (i8, i8) {
        (ge.lat2.u_shift_data.max_coord as i8, ge.lat2.u_shift_data.min_coord as i8)
    }

    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, s0: (i8, i8), bg_coord: BC, _r: BS::Item, idx: usize, v: bool) -> Option<(i8, i8)> {
        let mut min = s0.0 as isize;
        let mut max = s0.1 as isize;
        let (_, (u, _, _), _) = ge.lat2.spots[idx];

        if v != self.left_bg.bg_cell(bg_coord) {
            min = min.min(u);
        }
        if v != self.right_bg.bg_cell(bg_coord) {
            max = max.max(u);
        }
        // max >= min + (self.w.0 * shift_data.adet / self.w.1)
        if self.w.1 * max >= self.w.1 * min + self.w.0 * ge.lat1.adet {
            return None;
        }

        Some((min as i8, max as i8))
    }
}
