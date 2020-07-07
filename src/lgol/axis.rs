#![allow(unused_parens)]

use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;
use ars_ds::scalar::UScalar;

use crate::lgol;

use lgol::bg::LGolBg;
use lgol::bg::LGolBgCoord;
use lgol::graph::LGolHashNode;
use lgol::graph::RowTuple;
use lgol::lat2::LGolShiftData;

#[allow(dead_code)]
#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(PartialEq)]
pub enum LGolEdgeRead {
    Known(bool),
    Update(isize),
    Wrap,
    Unknown,
}

pub trait LGolRecenter {
    fn apply<BS: RowTuple, BC: LGolBgCoord, A: LGolAxis<BC>>(&self, a: &A, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>);
}

pub struct LGolRecenterCentered();

impl LGolRecenter for LGolRecenterCentered {
    fn apply<BS: RowTuple, BC: LGolBgCoord, A: LGolAxis<BC>>(&self, a: &A, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        a.recenter(shift_data, hn)
    }
}

pub struct LGolRecenterJustify();

impl LGolRecenter for LGolRecenterJustify {
    fn apply<BS: RowTuple, BC: LGolBgCoord, A: LGolAxis<BC>>(&self, a: &A, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        a.justify(shift_data, hn)
    }
}

pub trait LGolAxis<BC: LGolBgCoord>: Copy {
    type S: Nice;

    fn left_edge(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, c: isize) -> LGolEdgeRead;
    fn right_edge(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, c: isize) -> LGolEdgeRead;

    fn zero_stat(&self, shift_data: &LGolShiftData<BC>) -> Self::S;
    fn add_stat<B: UScalar>(&self, shift_data: &LGolShiftData<BC>, s0: Self::S, bg_coord: BC, r: B, idx: usize, c: isize, v: bool) -> Option<Self::S>;

    fn recenter<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>);
    fn justify<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>);

    fn wrap_in_print(&self) -> bool;
}

impl<BC: LGolBgCoord> LGolAxis<BC> for (LGolEdgeRead, LGolEdgeRead) {
    type S = ();

    fn left_edge(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, _c: isize) -> LGolEdgeRead {
        self.0
    }

    fn right_edge(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, _c: isize) -> LGolEdgeRead {
        self.1
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData<BC>) {
    }

    fn add_stat<B: UScalar>(&self, _shift_data: &LGolShiftData<BC>, _s0: (), _bg_coord: BC, _r: B, _idx: usize, _c: isize, _v: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        (0, hn)
    }

    fn justify<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        // TODO: maybe for this type of axis we should justify?
        (0, hn)
    }

    fn wrap_in_print(&self) -> bool {
        self == &(LGolEdgeRead::Wrap, LGolEdgeRead::Wrap)
    }
}

pub trait LGolEdge<BC>: Copy {
    fn left_edge(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, c: isize) -> LGolEdgeRead;
    fn right_edge(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, c: isize) -> LGolEdgeRead;
    fn is_wrap(&self) -> bool;
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolSimpleEdge(pub LGolEdgeRead);

impl<BC> LGolEdge<BC> for LGolSimpleEdge {
    fn left_edge(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, _c: isize) -> LGolEdgeRead {
        self.0
    }

    fn right_edge(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, _c: isize) -> LGolEdgeRead {
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
    fn left_edge(&self, _shift_data: &LGolShiftData<BC>, bg_coord: BC, _c: isize) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.0.bg_cell(bg_coord))
    }

    fn right_edge(&self, _shift_data: &LGolShiftData<BC>, bg_coord: BC, _c: isize) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.0.bg_cell(bg_coord))
    }

    fn is_wrap(&self) -> bool {
        false
    }
}

// self.0 is reflection axis in half units over the edge (0 is odd, 1 is even, 2 is gutter)
#[derive(Clone)]
#[derive(Copy)]
pub struct LGolReflectEdge(pub isize);

impl<BC> LGolEdge<BC> for LGolReflectEdge {
    fn left_edge(&self, shift_data: &LGolShiftData<BC>, _bg_coord: BC, c: isize) -> LGolEdgeRead {
        let mut c = c;

        // First, flip if appropriate
        // condition in rationals is: c < shift_data.min_coord - self.0 / 2
        let double_crit = 2 * shift_data.min_coord - self.0;
        if 2 * c < double_crit {
            c = double_crit - c;
        }

        // Now if c is still OOB we're in the gutter
        if c < shift_data.min_coord {
            // "read the background"
            return LGolEdgeRead::Known(false);
        }

        LGolEdgeRead::Update(c)
    }

    fn right_edge(&self, shift_data: &LGolShiftData<BC>, _bg_coord: BC, c: isize) -> LGolEdgeRead {
        let mut c = c;

        // condition is: c > shift_data.max_coord + self.0 / 2
        let double_crit = 2 * shift_data.max_coord + self.0;
        if 2 * c > double_crit {
            c = double_crit - c;
        }

        if c > shift_data.max_coord {
            return LGolEdgeRead::Known(false);
        }

        LGolEdgeRead::Update(c)
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

    fn left_edge(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, c: isize) -> LGolEdgeRead {
        self.left_edge.left_edge(shift_data, bg_coord, c)
    }

    fn right_edge(&self, shift_data: &LGolShiftData<BC>, bg_coord: BC, c: isize) -> LGolEdgeRead {
        self.right_edge.right_edge(shift_data, bg_coord, c)
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData<BC>) {
    }

    fn add_stat<B: UScalar>(&self, _shift_data: &LGolShiftData<BC>, _s0: (), _bg_coord: BC, _r: B, _idx: usize, _c: isize, _v: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        (0, hn)
    }

    fn justify<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        // TODO: maybe for this type of axis we should justify?
        (0, hn)
    }

    fn wrap_in_print(&self) -> bool {
        self.left_edge.is_wrap() && self.right_edge.is_wrap()
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolFancyAxis<LBG, RBG> {
    pub left_bg: LBG,
    pub right_bg: RBG,
}

impl<LBG, RBG> LGolFancyAxis<LBG, RBG> {
    fn shift<BS: RowTuple, BC: LGolBgCoord>(&self, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>, delta: isize) -> LGolHashNode<BS, BC> where LBG: LGolBg<BC>, RBG: LGolBg<BC> {
        if delta == 0 {
            return hn;
        }

        // update bg_coord to reflect new position
        let bg_coord = hn.bg_coord.add(shift_data.bg_period.mul(delta));

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
                            b = hn.rs.as_slice()[j].get_bit(idx2);
                        }
                    }

                    if b {
                        rss.as_slice_mut()[j].set_bit(idx, true);
                    }
                }
            }
        }

        LGolHashNode {
            bg_coord: bg_coord,
            rs: rss,
        }
    }
}

impl<BC: LGolBgCoord, LBG: LGolBg<BC>, RBG: LGolBg<BC>> LGolAxis<BC> for LGolFancyAxis<LBG, RBG> {
    type S = ();

    fn left_edge(&self, _shift_data: &LGolShiftData<BC>, bg_coord: BC, _c: isize) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.left_bg.bg_cell(bg_coord))
    }

    fn right_edge(&self, _shift_data: &LGolShiftData<BC>, bg_coord: BC, _c: isize) -> LGolEdgeRead {
        LGolEdgeRead::Known(self.right_bg.bg_cell(bg_coord))
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData<BC>) {
    }

    fn add_stat<B: UScalar>(&self, _shift_data: &LGolShiftData<BC>, _s0: (), _bg_coord: BC, _r: B, _idx: usize, _c: isize, _v: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        let min = self.left_bg.find_min(shift_data, &hn);
        let max = self.right_bg.find_max(shift_data, &hn);

        let our_sum = min + max;
        let def_sum = shift_data.min_coord + shift_data.max_coord;

        let delta = our_sum - def_sum;
        let delta = (delta + shift_data.period).div_euclid(2 * shift_data.period);

        (delta, self.shift(shift_data, hn, delta))
    }

    fn justify<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        let min = self.left_bg.find_min(shift_data, &hn);

        let def_min = shift_data.min_coord;

        let delta = min - def_min;
        let delta = delta.div_euclid(shift_data.period);

        (delta, self.shift(shift_data, hn, delta))
    }

    fn wrap_in_print(&self) -> bool {
        false
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolPeriodDividingAxis {
    pub division: usize,
    pub mf: u8,
}

impl<BC: LGolBgCoord> LGolAxis<BC> for LGolPeriodDividingAxis {
    type S = u8;

    fn left_edge(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, _c: isize) -> LGolEdgeRead {
        LGolEdgeRead::Wrap
    }

    fn right_edge(&self, _shift_data: &LGolShiftData<BC>, _bg_coord: BC, _c: isize) -> LGolEdgeRead {
        LGolEdgeRead::Wrap
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData<BC>) -> u8 {
        0
    }

    fn add_stat<B: UScalar>(&self, shift_data: &LGolShiftData<BC>, s0: u8, _bg_coord: BC, r: B, idx: usize, _c: isize, v: bool) -> Option<u8> {
        let mut idx1 = idx;
        let mut first = true;
        let division_walk = shift_data.division_walks[self.division].as_ref().unwrap();
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

    fn recenter<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        (0, hn)
    }

    fn justify<BS: RowTuple>(&self, _shift_data: &LGolShiftData<BC>, hn: LGolHashNode<BS, BC>) -> (isize, LGolHashNode<BS, BC>) {
        // TODO: maybe for this type of axis we should justify?
        (0, hn)
    }

    fn wrap_in_print(&self) -> bool {
        true
    }
}
