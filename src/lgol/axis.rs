#![allow(unused_parens)]

use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;

use crate::lgol;

use lgol::bg::LGolBg;
use lgol::bg::LGolBgCoord;
use lgol::graph::RowTuple;
use lgol::lat2::LGolShiftData;

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(PartialEq)]
pub enum LGolEdgeRead {
    Known(bool),
    Wrap,
    Unknown,
}

pub trait LGolAxis<BC: LGolBgCoord>: Copy {
    type S: Nice;

    fn left_edge(&self, bg_coord: BC) -> LGolEdgeRead;
    fn right_edge(&self, bg_coord: BC) -> LGolEdgeRead;

    fn zero_stat(&self, shift_data: &LGolShiftData<BC>) -> Self::S;
    fn add_stat(&self, shift_data: &LGolShiftData<BC>, s0: Self::S, bg_coord: BC, c: isize, v: bool) -> Option<Self::S>;

    fn recenter<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, rs: BS) -> (isize, BS);

    fn wrap_in_print(&self) -> bool;
}

impl<BC: LGolBgCoord> LGolAxis<BC> for (LGolEdgeRead, LGolEdgeRead) {
    type S = ();

    fn left_edge(&self, _bg_coord: BC) -> LGolEdgeRead {
        self.0
    }

    fn right_edge(&self, _bg_coord: BC) -> LGolEdgeRead {
        self.1
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData<BC>) {
    }

    fn add_stat(&self, _shift_data: &LGolShiftData<BC>, _s0: (), _bg_coord: BC, _c: isize, _v: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, rs: BS) -> (isize, BS) {
        (0, rs)
    }

    fn wrap_in_print(&self) -> bool {
        self == &(LGolEdgeRead::Wrap, LGolEdgeRead::Wrap)
    }
}

pub trait LGolEdge<BC>: Copy {
    fn edge(&self, bg_coord: BC) -> LGolEdgeRead;
    fn is_wrap(&self) -> bool;
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolSimpleEdge(pub LGolEdgeRead);

impl<BC> LGolEdge<BC> for LGolSimpleEdge {
    fn edge(&self, _bg_coord: BC) -> LGolEdgeRead {
        self.0
    }

    fn is_wrap(&self) -> bool {
        self.0 == LGolEdgeRead::Wrap
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolBgEdge<BG>(pub BG);

impl<BC: LGolBgCoord, BG: LGolBg<BC>> LGolEdge<BC> for LGolBgEdge<BG> {
    fn edge(&self, bg_coord: BC) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.0.bg_cell(bg_coord))
    }

    fn is_wrap(&self) -> bool {
        false
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolSimpleAxis<LE, RE> {
    pub left_edge: LE,
    pub right_edge: RE,
}

impl<BC: LGolBgCoord, LE: LGolEdge<BC>, RE: LGolEdge<BC>> LGolAxis<BC> for LGolSimpleAxis<LE, RE> {
    type S = ();

    fn left_edge(&self, bg_coord: BC) -> LGolEdgeRead {
        self.left_edge.edge(bg_coord)
    }

    fn right_edge(&self, bg_coord: BC) -> LGolEdgeRead {
        self.right_edge.edge(bg_coord)
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData<BC>) {
    }

    fn add_stat(&self, _shift_data: &LGolShiftData<BC>, _s0: (), _bg_coord: BC, _c: isize, _v: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, rs: BS) -> (isize, BS) {
        (0, rs)
    }

    fn wrap_in_print(&self) -> bool {
        self.left_edge.is_wrap() && self.right_edge.is_wrap()
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolFancyAxis<LBG, RBG> {
    // (numerator, denominator), a value of 1 is the entire width
    pub w: (isize, isize),
    pub left_bg: LBG,
    pub right_bg: RBG,
}

impl<LBG, RBG> LGolFancyAxis<LBG, RBG> {
    fn find_bs_min<BS: RowTuple, BC: LGolBgCoord>(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, rs: BS) -> isize where LBG: LGolBg<BC> {
        for &(c, idx, db) in shift_data.checks.iter() {
            let bg_coord = db.add(bg_coord);
            for (j, r) in rs.as_slice().iter().enumerate() {
                let bg_coord = bg_coord.add(shift_data.w_bg_coord.mul(-((j as isize) + 1)));
                let bg_cell = self.left_bg.bg_cell(bg_coord);
                if r.get_bit(idx) != bg_cell {
                    return c;
                }
            }
        }
        shift_data.max_coord
    }

    fn find_bs_max<BS: RowTuple, BC: LGolBgCoord>(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, rs: BS) -> isize where RBG: LGolBg<BC> {
        for &(c, idx, db) in shift_data.checks.iter().rev() {
            let bg_coord = db.add(bg_coord);
            for (j, r) in rs.as_slice().iter().enumerate() {
                let bg_coord = bg_coord.add(shift_data.w_bg_coord.mul(-((j as isize) + 1)));
                let bg_cell = self.right_bg.bg_cell(bg_coord);
                if r.get_bit(idx) != bg_cell {
                    return c;
                }
            }
        }
        shift_data.min_coord
    }
}

impl<BC: LGolBgCoord, LBG: LGolBg<BC>, RBG: LGolBg<BC>> LGolAxis<BC> for LGolFancyAxis<LBG, RBG> {
    type S = (isize, isize);

    fn left_edge(&self, bg_coord: BC) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.left_bg.bg_cell(bg_coord))
    }

    fn right_edge(&self, bg_coord: BC) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.right_bg.bg_cell(bg_coord))
    }

    fn zero_stat(&self, shift_data: &LGolShiftData<BC>) -> (isize, isize) {
        (shift_data.max_coord, shift_data.min_coord)
    }

    fn add_stat(&self, shift_data: &LGolShiftData<BC>, s0: (isize, isize), bg_coord: BC, c: isize, v: bool) -> Option<(isize, isize)> {
        let mut min = s0.0;
        let mut max = s0.1;

        if v != self.left_bg.bg_cell(bg_coord) {
            min = min.min(c);
        }
        if v != self.right_bg.bg_cell(bg_coord) {
            max = max.max(c);
        }
        // max >= min + (self.w.0 * shift_data.adet / self.w.1)
        if self.w.1 * max >= self.w.1 * min + self.w.0 * shift_data.adet {
            return None;
        }

        Some((min, max))
    }

    fn recenter<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, rs: BS) -> (isize, BS) {
        let min = self.find_bs_min(shift_data, bg_coord, rs);
        let max = self.find_bs_max(shift_data, bg_coord, rs);

        let our_sum = min + max;
        let def_sum = shift_data.min_coord + shift_data.max_coord;

        let delta = our_sum - def_sum;
        let delta = (delta + shift_data.period).div_euclid(2 * shift_data.period);

        if delta == 0 {
            return (0, rs);
        }

        // update bg_coord to reflect new position
        let bg_coord = bg_coord.add(shift_data.bg_period.mul(delta));

        let mut rss = BS::default();
        for shift_row in shift_data.shift_rows.iter() {
            for (i, &(idx, db)) in shift_row.iter().enumerate() {
                // update bg_coord for idx
                let bg_coord = bg_coord.add(db);

                let i2 = (i as isize) + delta;
                for j in 0..BS::len() {
                    // update bg_coord for (j + 1)
                    let bg_coord = bg_coord.add(shift_data.w_bg_coord.mul(-((j as isize) + 1)));

                    let b;
                    if i2 < 0 {
                        // read off left end of previous, fill with left BG
                        b = self.left_bg.bg_cell(bg_coord);
                    }
                    else {
                        let i2 = (i2 as usize);
                        if i2 >= shift_row.len() {
                            // read off right end of previous, fill with right BG
                            b = self.right_bg.bg_cell(bg_coord);
                        }
                        else {
                            // read from previous
                            let idx2 = shift_row[i2].0;
                            b = rs.as_slice()[j].get_bit(idx2);
                        }
                    }

                    if b {
                        rss.as_slice_mut()[j].set_bit(idx, true);
                    }
                }
            }
        }

        (delta, rss)
    }

    fn wrap_in_print(&self) -> bool {
        false
    }
}
