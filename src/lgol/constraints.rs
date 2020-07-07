#![allow(unused_parens)]

use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;

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
pub struct LGolConstraintUWindow<LBG, RBG> {
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

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolConstraintVPeriodDividing {
    pub division: usize,
    pub mf: u8,
}

impl<BC: LGolBgCoord> LGolConstraint<BC> for LGolConstraintVPeriodDividing {
    type S = u8;

    fn zero_stat<BS: RowTuple>(&self, _ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> u8 {
        0
    }

    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, s0: u8, _bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<u8> {
        let mut idx1 = idx;
        let mut first = true;
        let division_walk = ge.lat2.v_shift_data.division_walks[self.division].as_ref().unwrap();
        loop {
            idx1 = division_walk[idx1];
            if idx1 >= idx {
                break;
            }
            if r.get_bit(idx1) == v {
                // someone matched us already, we're definitely not charged
                return Some(s0);
            }
            first = false;
            continue;
        }
        if first {
            // actually we were the first
            return Some(s0);
        }
        // we're not first and everyone before us were all the opposite of us, we get charged
        let s1 = s0 + 1;
        if s1 > self.mf {
            return None;
        }
        Some(s1)
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolConstraintGreyshipMegahack<LBG, RBG> {
    pub f: (isize, isize),
    pub w: (isize, isize),
    pub left_bg: LBG,
    pub right_bg: RBG,
    pub division: usize,
}

impl<BC: LGolBgCoord, LBG: LGolBg<BC>, RBG: LGolBg<BC>> LGolConstraint<BC> for LGolConstraintGreyshipMegahack<LBG, RBG> {
    type S = Option<(i8, i8)>;

    fn zero_stat<BS: RowTuple>(&self, _ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Option<(i8, i8)> {
        None
    }

    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, s0: Option<(i8, i8)>, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Option<(i8, i8)>> {
        match s0 {
            None => {
                // haven't yet seen a non-left cell
                if v == self.left_bg.bg_cell(bg_coord) {
                    // still haven't
                    return Some(None);
                }
                // okay, we're the first non-left cell
                let (_, (min, _, _), _) = ge.lat2.spots[idx];

                let max1 = min + self.f.0 * ge.lat1.adet / self.f.1;
                let max2 = min + self.w.0 * ge.lat1.adet / self.w.1;

                Some(Some((max1 as i8, max2 as i8)))
            },
            Some((max1, max2)) => {
                // already seen a non-left cell, make sure we don't violate right side of windows
                let max1 = max1 as isize;
                let max2 = max2 as isize;

                let (_, (u, _, _), _) = ge.lat2.spots[idx];

                if u >= max1 {
                    // we're past the limit for full-period cells, check to make sure we're same as
                    // prior (where applicable)
                    let division_walk = ge.lat2.v_shift_data.division_walks[self.division].as_ref().unwrap();
                    let prev_idx = division_walk[idx];

                    if prev_idx < idx {
                        // we're not first, so check
                        if r.get_bit(prev_idx) != v {
                            // nope nope nope
                            return None;
                        }
                    }
                }

                if u >= max2 {
                    // we're past the limit for non-right cells
                    if v != self.right_bg.bg_cell(bg_coord) {
                        return None;
                    }
                }

                Some(s0)
            },
        }
    }
}
