#![allow(unused_parens)]

use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::hash::Hash;

use crate::lgol;

use lgol::graph::LGolHashNode;
use lgol::graph::RowTuple;
use lgol::lat1::Vec3;
use lgol::lat2::LGolShiftData;

pub trait LGolBgCoord: Nice + Default {
    fn mul(&self, n: isize) -> Self;
    fn add(&self, other: Self) -> Self;
    fn from_xyt(xyt: Vec3) -> Self;

    fn to_idx(&self) -> usize;
    fn from_idx(idx: usize) -> Self;
    fn max_idx() -> usize;
}

impl LGolBgCoord for () {
    fn mul(&self, _n: isize) {
    }

    fn add(&self, _other: ()) {
    }

    fn from_xyt(_xyt: Vec3) {
    }

    fn to_idx(&self) -> usize {
        0
    }

    fn from_idx(_idx: usize) {
    }

    fn max_idx() -> usize {
        return 1;
    }
}

pub trait LGolBgHasX2: LGolBgCoord {
    fn x2(&self) -> i8;
}

pub trait LGolBgHasY2: LGolBgCoord {
    fn y2(&self) -> i8;
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(Ord)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
#[derive(Serialize)]
pub struct LGolBgX2(pub i8);

impl LGolBgCoord for LGolBgX2 {
    fn mul(&self, n: isize) -> LGolBgX2 {
        LGolBgX2(((n as i8) * self.0).rem_euclid(2))
    }

    fn add(&self, other: LGolBgX2) -> LGolBgX2 {
        LGolBgX2((self.0 + other.0) % 2)
    }

    fn from_xyt((x, _y, _t): Vec3) -> LGolBgX2 {
        LGolBgX2(x.rem_euclid(2) as i8)
    }

    fn to_idx(&self) -> usize {
        self.0 as usize
    }

    fn from_idx(idx: usize) -> LGolBgX2 {
        LGolBgX2(idx as i8)
    }

    fn max_idx() -> usize {
        2
    }
}

impl LGolBgHasX2 for LGolBgX2 {
    fn x2(&self) -> i8 {
        self.0
    }
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(Ord)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
#[derive(Serialize)]
pub struct LGolBgY2(pub i8);

impl LGolBgCoord for LGolBgY2 {
    fn mul(&self, n: isize) -> LGolBgY2 {
        LGolBgY2(((n as i8) * self.0).rem_euclid(2))
    }

    fn add(&self, other: LGolBgY2) -> LGolBgY2 {
        LGolBgY2((self.0 + other.0) % 2)
    }

    fn from_xyt((_x, y, _t): Vec3) -> LGolBgY2 {
        LGolBgY2(y.rem_euclid(2) as i8)
    }

    fn to_idx(&self) -> usize {
        self.0 as usize
    }

    fn from_idx(idx: usize) -> LGolBgY2 {
        LGolBgY2(idx as i8)
    }

    fn max_idx() -> usize {
        2
    }
}

impl LGolBgHasY2 for LGolBgY2 {
    fn y2(&self) -> i8 {
        self.0
    }
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(Ord)]
#[derive(PartialEq)]
#[derive(PartialOrd)]
#[derive(Serialize)]
pub struct LGolBgX2Y2(pub i8);

impl LGolBgX2Y2 {
    fn split(&self) -> (i8, i8) {
        let x = self.0 % 2;
        let y = self.0 / 2;
        (x, y)
    }

    fn pair(x: i8, y: i8) -> LGolBgX2Y2 {
        let x = x.rem_euclid(2);
        let y = y.rem_euclid(2);
        LGolBgX2Y2(2 * y + x)
    }
}

impl LGolBgCoord for LGolBgX2Y2 {
    fn mul(&self, n: isize) -> LGolBgX2Y2 {
        let (x, y) = self.split();
        let x = (n as i8) * x;
        let y = (n as i8) * y;
        LGolBgX2Y2::pair(x, y)
    }

    fn add(&self, other: LGolBgX2Y2) -> LGolBgX2Y2 {
        let (x1, y1) = self.split();
        let (x2, y2) = other.split();
        LGolBgX2Y2::pair(x1 + x2, y1 + y2)
    }

    fn from_xyt((x, y, _t): Vec3) -> LGolBgX2Y2 {
        LGolBgX2Y2::pair(x as i8, y as i8)
    }

    fn to_idx(&self) -> usize {
        self.0 as usize
    }

    fn from_idx(idx: usize) -> LGolBgX2Y2 {
        LGolBgX2Y2(idx as i8)
    }

    fn max_idx() -> usize {
        4
    }
}

impl LGolBgHasX2 for LGolBgX2Y2 {
    fn x2(&self) -> i8 {
        let (x, _y) = self.split();
        x
    }
}

impl LGolBgHasY2 for LGolBgX2Y2 {
    fn y2(&self) -> i8 {
        let (_x, y) = self.split();
        y
    }
}

pub trait LGolBg<BC: LGolBgCoord>: Copy {
    fn bg_cell(&self, bg_coord: BC) -> bool;

    fn find_min<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, hn: &LGolHashNode<BS, BC>) -> isize {
        for &(c, idx, db) in shift_data.checks.iter() {
            let bg_coord = db.add(hn.bg_coord);
            for (j, r) in hn.rs.as_slice().iter().enumerate() {
                let bg_coord = bg_coord.add(shift_data.w_bg_coord.mul(-((j as isize) + 1)));
                let bg_cell = self.bg_cell(bg_coord);
                if r.get_bit(idx) != bg_cell {
                    return c;
                }
            }
        }
        shift_data.max_coord
    }

    fn find_max<BS: RowTuple>(&self, shift_data: &LGolShiftData<BC>, hn: &LGolHashNode<BS, BC>) -> isize {
        for &(c, idx, db) in shift_data.checks.iter().rev() {
            let bg_coord = db.add(hn.bg_coord);
            for (j, r) in hn.rs.as_slice().iter().enumerate() {
                let bg_coord = bg_coord.add(shift_data.w_bg_coord.mul(-((j as isize) + 1)));
                let bg_cell = self.bg_cell(bg_coord);
                if r.get_bit(idx) != bg_cell {
                    return c;
                }
            }
        }
        shift_data.min_coord
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolBgEmpty();

impl<BC: LGolBgCoord> LGolBg<BC> for LGolBgEmpty {
    fn bg_cell(&self, _bg_coord: BC) -> bool {
        false
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolBgVertStripes();

impl<BC: LGolBgHasX2> LGolBg<BC> for LGolBgVertStripes {
    fn bg_cell(&self, bg_coord: BC) -> bool {
        bg_coord.x2() == 0
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolBgHorizStripes();

impl<BC: LGolBgHasY2> LGolBg<BC> for LGolBgHorizStripes {
    fn bg_cell(&self, bg_coord: BC) -> bool {
        bg_coord.y2() == 0
    }
}
