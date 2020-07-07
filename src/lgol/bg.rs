#![allow(unused_parens)]

use ars_ds::nice::Nice;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::hash::Hash;

use crate::lgol;

use lgol::lat1::Vec3;

pub trait LGolBgCoord: Nice + Default + Serialize {
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

pub trait LGolBg<BC: LGolBgCoord>: Copy {
    fn bg_cell(&self, bg_coord: BC) -> bool;
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

impl LGolBg<LGolBgX2> for LGolBgVertStripes {
    fn bg_cell(&self, bg_coord: LGolBgX2) -> bool {
        bg_coord.0 == 0
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolBgHorizStripes();

impl LGolBg<LGolBgY2> for LGolBgHorizStripes {
    fn bg_cell(&self, bg_coord: LGolBgY2) -> bool {
        bg_coord.0 == 0
    }
}
