#![allow(unused_parens)]

use ars_aa::lattice::LatticeCanonicalizable;
use ars_aa::lattice::LatticeCanonicalizer;
use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;
use ars_ds::scalar::UScalar;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::dfs;
use crate::gol;
use crate::lgol;

use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use gol::printbag::PrintBag;
use lgol::axis::LGolAxis;
use lgol::axis::LGolEdgeRead;
use lgol::bg::LGolBgCoord;
use lgol::ends::LGolEnds;
use lgol::lat1::LGolLat1;
use lgol::lat1::Vec3;
use lgol::lat2::LGolLat2;

pub trait RowTuple: Copy + Debug + Default + Eq + Hash + Send + Sync {
    type Item: UScalar;

    fn len() -> usize;
    fn as_slice(&self) -> &[Self::Item];
    fn as_slice_mut(&mut self) -> &mut [Self::Item];
    fn from_slice(slice: &[Self::Item]) -> Self;
}

macro_rules! impl_row_tuple {
    ($n:expr) => {
        impl<B: UScalar> RowTuple for [B; $n] {
            type Item = B;

            fn len() -> usize {
                $n
            }

            fn as_slice(&self) -> &[B] {
                self
            }

            fn as_slice_mut(&mut self) -> &mut [B] {
                self
            }

            fn from_slice(slice: &[B]) -> [B; $n] {
                let mut r = [B::default(); $n];
                r.copy_from_slice(slice);
                r
            }
        }
    }
}

impl_row_tuple!(0);
impl_row_tuple!(1);
impl_row_tuple!(2);
impl_row_tuple!(3);
impl_row_tuple!(4);
impl_row_tuple!(5);
impl_row_tuple!(6);
impl_row_tuple!(7);
impl_row_tuple!(8);
impl_row_tuple!(9);
impl_row_tuple!(10);
impl_row_tuple!(11);
impl_row_tuple!(12);
impl_row_tuple!(13);
impl_row_tuple!(14);
impl_row_tuple!(15);
impl_row_tuple!(16);
impl_row_tuple!(17);
impl_row_tuple!(18);
impl_row_tuple!(19);
impl_row_tuple!(20);
impl_row_tuple!(21);
impl_row_tuple!(22);
impl_row_tuple!(23);
impl_row_tuple!(24);
impl_row_tuple!(25);
impl_row_tuple!(26);
impl_row_tuple!(27);
impl_row_tuple!(28);
impl_row_tuple!(29);
impl_row_tuple!(30);
impl_row_tuple!(31);
impl_row_tuple!(32);

#[derive(Clone)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
#[derive(Serialize)]
pub struct LGolNode<BS: RowTuple, BC, US, VS> {
    pub bg_coord: BC,
    pub du: i16,
    pub dv: i16,
    pub r0s: BS,
    pub r1: BS::Item,
    pub r1l: u8,
    pub r1_us: US,
    pub r1_vs: VS,
}

impl<BS: RowTuple, BC: Nice, US: Nice, VS: Nice> DfsNode for LGolNode<BS, BC, US, VS> {
    type KN = LGolKeyNode<BS, BC>;

    fn key_node(&self) -> Option<LGolKeyNode<BS, BC>> {
        if self.r1l != 0 {
            return None;
        }

        Some(LGolKeyNode {
            bg_coord: self.bg_coord,
            du: self.du,
            dv: self.dv,
            rs: self.r0s,
        })
    }
}

impl<BS: RowTuple, BC: Nice, US: Nice, VS: Nice> LGolNode<BS, BC, US, VS> {
    fn read_mask(&self, (idx, mask): (usize, BS::Item)) -> u32 {
        let r = match idx {
            0 => self.r1,
            _ => self.r0s.as_slice()[idx - 1],
        };
        (r & mask).count_ones()
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
#[derive(Serialize)]
pub struct LGolKeyNode<BS: RowTuple, BC> {
    pub bg_coord: BC,
    pub du: i16,
    pub dv: i16,
    pub rs: BS,
}

impl<BS: RowTuple, BC: Nice> LGolKeyNode<BS, BC> {
    pub fn lgol_hash_node(&self) -> LGolHashNode<BS, BC> {
        LGolHashNode {
            bg_coord: self.bg_coord,
            rs: self.rs,
        }
    }
}

impl<BS: RowTuple, BC: Nice> DfsKeyNode for LGolKeyNode<BS, BC> {
    type HN = LGolHashNode<BS, BC>;

    fn hash_node(&self) -> Option<LGolHashNode<BS, BC>> {
        Some(self.lgol_hash_node())
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct LGolHashNode<BS: RowTuple, BC> {
    pub bg_coord: BC,
    pub rs: BS,
}

#[derive(Clone)]
pub struct LGolGraphParams<BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>> {
    pub vu: Vec3,
    pub vv: Vec3,
    pub vw: Vec3,

    pub bg_coord: PhantomData<BC>,

    pub u_axis: UA,
    pub v_axis: VA,
}

enum PartialRowRead {
    Known(bool),
    Unknown,
    Read(usize, usize),
}

impl<BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>> LGolGraphParams<BC, UA, VA> {
    pub fn derived<BS: RowTuple, E: LGolEnds<BS, BC>>(&self, ends: E) -> LGolGraph<BS, BC, UA, VA, E> {
        let lat1 = LGolLat1::new(self.vu, self.vv, self.vw);
        let lat2 = LGolLat2::new(&lat1);

        let xyt_idx = lat2.spots.iter().enumerate().map(|(idx, &(xyt, _uvw, _))| (xyt, idx)).collect::<HashMap<_, _>>();

        let compute_prow_read = |bg_coord: BC, rl, xyt| {
            let bg_coord = bg_coord.add(BC::from_xyt(xyt));
            let (xyt, (lu, lv, lw)) = lat1.canonicalize_xyt(xyt);

            let u_edge;
            if lu < 0 {
                u_edge = self.u_axis.left_edge(bg_coord);
            }
            else if lu > 0 {
                u_edge = self.u_axis.right_edge(bg_coord);
            }
            else {
                u_edge = LGolEdgeRead::Wrap;
            }

            let v_edge;
            if lv < 0 {
                v_edge = self.v_axis.left_edge(bg_coord);
            }
            else if lv > 0 {
                v_edge = self.v_axis.right_edge(bg_coord);
            }
            else {
                v_edge = LGolEdgeRead::Wrap;
            }

            let edge = match u_edge {
                LGolEdgeRead::Wrap => v_edge,
                _ => match v_edge {
                    LGolEdgeRead::Wrap => u_edge,
                    _ => panic!("Edge conflict at {:?}, u_edge {:?}, v_edge {:?}", xyt, u_edge, v_edge),
                }
            };

            match edge {
                LGolEdgeRead::Known(b) => PartialRowRead::Known(b),
                LGolEdgeRead::Wrap => {
                    let idx = *xyt_idx.get(&xyt).unwrap();

                    if lw > 0 {
                        // not yet building
                        PartialRowRead::Unknown
                    }
                    else if lw < 0 {
                        // already built, will necessarily be able to read
                        PartialRowRead::Read((-lw) as usize, idx)
                    }
                    else {
                        // in the current row, see if it will already be set
                        if idx < rl {
                            PartialRowRead::Read(0, idx)
                        }
                        else {
                            PartialRowRead::Unknown
                        }
                    }
                }
                LGolEdgeRead::Unknown => PartialRowRead::Unknown,
            }
        };

        let single_mask = |idx| {
            let mut b = BS::Item::zero();
            b.set_bit(idx, true);
            b
        };

        let compute_checks = |bg_coord, idx, (x, y, t)| {
            let rl = idx + 1;

            let mut checks = Vec::new();

            let mut f = |dx, dy, dt| {
                let x2 = x + dx;
                let y2 = y + dy;
                let t2 = t + dt;

                let cur_mask = match compute_prow_read(bg_coord, rl, (x2, y2, t2)) {
                    PartialRowRead::Known(b) => (0, BS::Item::zero(), b),
                    PartialRowRead::Unknown => {
                        return;
                    },
                    PartialRowRead::Read(row_idx, bit_idx) => (row_idx, single_mask(bit_idx), false),
                };
                let fut_mask = match compute_prow_read(bg_coord, rl, (x2, y2, t2 + 1)) {
                    PartialRowRead::Known(b) => (0, BS::Item::zero(), b),
                    PartialRowRead::Unknown => {
                        return;
                    },
                    PartialRowRead::Read(row_idx, bit_idx) => (row_idx, single_mask(bit_idx), false),
                };

                let mut nh_masks: Vec<(usize, BS::Item)> = Vec::new();
                let mut nh_live = 0;
                let mut nh_ct = 0;

                for dx2 in -1..=1 {
                    'dy2: for dy2 in -1..=1 {
                        if (dx2, dy2) == (0, 0) {
                            continue;
                        }

                        let x3 = x2 + dx2;
                        let y3 = y2 + dy2;
                        let t3 = t2;

                        match compute_prow_read(bg_coord, rl, (x3, y3, t3)) {
                            PartialRowRead::Known(b) => {
                                if b {
                                    nh_live += 1;
                                }
                                nh_ct += 1;
                            }
                            PartialRowRead::Unknown => {
                            }
                            PartialRowRead::Read(row_idx, bit_idx) => {
                                nh_ct += 1;

                                for &mut (nh_idx, ref mut nh_mask) in nh_masks.iter_mut() {
                                    if nh_idx != row_idx {
                                        continue;
                                    }
                                    if !nh_mask.get_bit(bit_idx) {
                                        nh_mask.set_bit(bit_idx, true);
                                        continue 'dy2;
                                    }
                                }
                                nh_masks.push((row_idx, single_mask(bit_idx)));
                            }
                        }
                    }
                }

                checks.push((nh_masks, nh_live, nh_ct, cur_mask, fut_mask));
            };

            for dx in -1..=1 {
                for dy in -1..=1 {
                    f(dx, dy, 0);
                }
            }
            f(0, 0, -1);

            checks
        };

        let checks: HashMap<_, _> = BC::all().into_iter().map(|bg_coord| {
            let checks: Vec<_> = lat2.spots.iter().enumerate().map(|(idx, &(xyt, _, _))| compute_checks(bg_coord, idx, xyt)).collect();

            (bg_coord, checks)
        }).collect();

        let max_row_idx = checks.iter().map(|(_bg_coord, checks)| {
            checks.iter().map(|checks| {
                checks.iter().map(|&(ref nh_masks, _, _, _, _)| {
                    nh_masks.iter().map(|&(row_idx, _)| row_idx)
                }).flatten()
            }).flatten()
        }).flatten().max().unwrap();
        assert!(max_row_idx <= BS::len(), "{} <= {}", max_row_idx, BS::len());

        let max_r1l = (lat1.adet as usize);

        LGolGraph {
            params: self.clone(),
            lat1: lat1,
            lat2: lat2,

            max_r1l: max_r1l,
            checks: checks,

            ends: ends,

            _bs: PhantomData::default(),
        }
    }
}

pub struct LGolGraph<BS: RowTuple, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, E: LGolEnds<BS, BC>> {
    pub params: LGolGraphParams<BC, UA, VA>,
    pub lat1: LGolLat1,
    pub lat2: LGolLat2<BC>,

    pub max_r1l: usize,
    pub checks: HashMap<BC, Vec<Vec<(Vec<(usize, BS::Item)>, u32, u32, (usize, BS::Item, bool), (usize, BS::Item, bool))>>>,

    pub ends: E,

    _bs: PhantomData<BS>,
}

impl<BS: RowTuple, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, E: LGolEnds<BS, BC>> LGolGraph<BS, BC, UA, VA, E> {
    fn recenter(&self, bg_coord: BC, rs: BS) -> (isize, isize, BC, BS) {
        let (su, rs) = self.params.u_axis.recenter(&self.lat2.u_shift_data, bg_coord, rs);
        let bg_coord = bg_coord.add(self.lat2.u_shift_data.bg_period.mul(su));

        let (sv, rs) = self.params.v_axis.recenter(&self.lat2.v_shift_data, bg_coord, rs);
        let bg_coord = bg_coord.add(self.lat2.v_shift_data.bg_period.mul(sv));

        let du = su * self.lat2.u_shift_data.period;
        let dv = sv * self.lat2.v_shift_data.period;
        (du, dv, bg_coord, rs)
    }

    fn expand_srch(&self, n1: &LGolNode<BS, BC, UA::S, VA::S>, n2s: &mut Vec<LGolNode<BS, BC, UA::S, VA::S>>) {
        let idx = n1.r1l as usize;

        if idx == self.max_r1l {
            let mut r0s = BS::default();

            r0s.as_slice_mut()[1..BS::len()].copy_from_slice(&n1.r0s.as_slice()[0..(BS::len() - 1)]);
            r0s.as_slice_mut()[0] = n1.r1;
            let bg_coord = n1.bg_coord.add(self.lat2.w_bg_coord);

            let (du, dv, bg_coord, r0s) = self.recenter(bg_coord, r0s);

            if n1.r0s == BS::default() && (du, dv) != (0, 0) {
                // reject stupid first row shifts
                return;
            }

            n2s.push(LGolNode {
                bg_coord: bg_coord,
                du: n1.du + (du as i16),
                dv: n1.dv + (dv as i16),
                r0s: r0s,
                r1: BS::Item::zero(),
                r1_us: self.params.u_axis.zero_stat(&self.lat2.u_shift_data),
                r1_vs: self.params.v_axis.zero_stat(&self.lat2.v_shift_data),
                r1l: 0,
            });
            return;
        }

        let (_, (idx_u, idx_v, _), idx_bg_coord) = self.lat2.spots[idx];
        let v_bg_coord = n1.bg_coord.add(idx_bg_coord);

        'v: for &v in &[false, true] {
            let mut n2 = LGolNode {
                bg_coord: n1.bg_coord,
                du: n1.du,
                dv: n1.dv,
                r0s: n1.r0s,
                r1: n1.r1,
                r1l: n1.r1l + 1,
                r1_us: match self.params.u_axis.add_stat(&self.lat2.u_shift_data, n1.r1_us, v_bg_coord, idx_u, v) {
                    Some(us) => us,
                    None => {
                        continue 'v;
                    }
                },
                r1_vs: match self.params.v_axis.add_stat(&self.lat2.v_shift_data, n1.r1_vs, v_bg_coord, idx_v, v) {
                    Some(us) => us,
                    None => {
                        continue 'v;
                    }
                },
            };

            n2.r1.set_bit(idx, v);

            let checks = self.checks.get(&n2.bg_coord).unwrap();
            let checks = &checks[idx];
            for &(ref nh_masks, nh_live, nh_ct, cur_mask, fut_mask) in checks.iter() {
                let mut nh = nh_live;
                for &nh_mask in nh_masks {
                    nh += n2.read_mask(nh_mask);
                }

                let (cur_row_idx, cur_mask, cur_force) = cur_mask;
                let (fut_row_idx, fut_mask, fut_force) = fut_mask;
                let cur_cell = match cur_force {
                    false => (n2.read_mask((cur_row_idx, cur_mask)) != 0),
                    true => true,
                };
                let fut_cell = match fut_force {
                    false => (n2.read_mask((fut_row_idx, fut_mask)) != 0),
                    true => true,
                };

                if !check_compat2(nh, nh_ct, cur_cell, fut_cell) {
                    continue 'v;
                }
            }

            n2s.push(n2.clone());
        }
    }

    fn collect_row(&self, pr: &mut PrintBag, c0: char, c1: char, row: BS::Item, rl: Option<usize>, du: isize, dv: isize, w: isize) {
        // awkwardly w is in actual steps of w while du and dv are in units of 1/|det|
        let (dx, dy, dt) = self.lat1.uvw_to_xyt((du, dv, self.lat1.adet * w));

        let mut wraps = vec![];
        if self.params.u_axis.wrap_in_print() {
            wraps.push(self.params.vu);
        }
        if self.params.v_axis.wrap_in_print() {
            wraps.push(self.params.vv);
        }
        let wraps = Vec3::canonicalize(wraps);

        for (idx, &((x, y, t), _uvw, _)) in self.lat2.spots.iter().enumerate() {
            let x = x + dx;
            let y = y + dy;
            let t = t + dt;

            let mut c = match row.get_bit(idx) {
                true => c1,
                false => c0,
            };
            if let Some(rl) = rl {
                if idx >= rl {
                    c = '?';
                }
            }
            let (x, y, t) = wraps.canonicalize((x, y, t));
            pr.insert(x, y, t, c);
        }
    }

    pub fn format_rows(&self, rows: &Vec<LGolKeyNode<BS, BC>>, last: Option<&LGolNode<BS, BC, UA::S, VA::S>>) -> Vec<String> {
        let mut pr = PrintBag::new();
        let mut w = 0;
        for (n, row) in rows.iter().enumerate() {
            let du = row.du as isize;
            let dv = row.dv as isize;
            if n == 0 {
                for &r in row.rs.as_slice()[1..BS::len()].iter().rev() {
                    self.collect_row(&mut pr, '.', '*', r, None, du, dv, w);
                    w += 1;
                }
            }
            self.collect_row(&mut pr, '.', '*', row.rs.as_slice()[0], None, du, dv, w);
            w += 1;
        }
        if let Some(last) = last {
            self.collect_row(&mut pr, '.', '*', last.r1, Some(last.r1l as usize), (last.du as isize), (last.dv as isize), w);
        }
        pr.format()
    }

    pub fn format_cycle_rows(&self, path: &Vec<LGolKeyNode<BS, BC>>, cycle: &Vec<LGolKeyNode<BS, BC>>, _last: &LGolKeyNode<BS, BC>) -> Vec<String> {
        let mut pr = PrintBag::new();
        let mut w = 0;
        let last = BS::len() - 1;
        for row in path.iter() {
            self.collect_row(&mut pr, '.', '*', row.rs.as_slice()[last], None, (row.du as isize), (row.dv as isize), w);
            w += 1;
        }
        for row in cycle.iter() {
            self.collect_row(&mut pr, 'x', 'o', row.rs.as_slice()[last], None, (row.du as isize), (row.dv as isize), w);
            w += 1;
        }
        pr.format()
    }

    pub fn zero_node(&self) -> LGolNode<BS, BC, UA::S, VA::S> {
        self.regular_node((0, 0, 0), BS::default())
    }

    pub fn parse_bs<S: AsRef<str>>(&self, rs: impl IntoIterator<Item=S>) -> BS {
        let mut rs: Vec<_> = rs.into_iter().map(|s| {
            let s = s.as_ref();

            let mut r = BS::Item::zero();
            for (idx, c) in s.chars().enumerate() {
                assert!(idx < self.max_r1l);
                let b = match c {
                    '.' => false,
                    '*' => true,
                    _ => panic!(),
                };
                r.set_bit(idx, b);
            }
            r
        }).collect();
        rs.reverse();
        BS::from_slice(&rs)
    }

    pub fn recenter_xyt(&self, xyt: Vec3, rs: BS) -> (Vec3, BS) {
        let bg_coord = BC::from_xyt(xyt);
        let (du, dv, _bg_coord, rs) = self.recenter(bg_coord, rs);
        let xyt = self.lat1.uvw_to_xyt((du, dv, 0));
        (xyt, rs)
    }

    pub fn regular_node(&self, xyt: Vec3, r0s: BS) -> LGolNode<BS, BC, UA::S, VA::S> {
        let (u, v, w) = self.lat1.xyt_to_uvw(xyt);

        assert_eq!(w, 0);

        LGolNode {
            bg_coord: BC::from_xyt(xyt),
            du: (u as i16),
            dv: (v as i16),
            r0s: r0s,
            r1: BS::Item::zero(),
            r1l: 0,
            r1_us: self.params.u_axis.zero_stat(&self.lat2.u_shift_data),
            r1_vs: self.params.v_axis.zero_stat(&self.lat2.v_shift_data),
        }
    }

    pub fn key_node(&self, xyt: Vec3, rs: BS) -> LGolKeyNode<BS, BC> {
        let (u, v, w) = self.lat1.xyt_to_uvw(xyt);

        assert_eq!(w, 0);

        LGolKeyNode {
            bg_coord: BC::from_xyt(xyt),
            du: (u as i16),
            dv: (v as i16),
            rs: rs,
        }
    }
}

fn check_compat2(living: u32, known: u32, c: bool, f: bool) -> bool {
    let dead = known - living;
    match c {
        true => match f {
            // need 2 or 3
            true => (living <= 3 && dead <= 6),
            // need 0, 1, or 4+
            false => (living <= 1 || dead <= 4),
        },
        false => match f {
            // need 3
            true => (living <= 3 && dead <= 5),
            false => (living <= 2 || dead <= 4),
        },
    }
}

impl<BS: RowTuple, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, E: LGolEnds<BS, BC>> DfsGraph<LGolNode<BS, BC, UA::S, VA::S>> for LGolGraph<BS, BC, UA, VA, E> {
    fn expand(&self, n1: &LGolNode<BS, BC, UA::S, VA::S>) -> Vec<LGolNode<BS, BC, UA::S, VA::S>> {
        let mut n2s = Vec::new();
        self.expand_srch(n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &LGolKeyNode<BS, BC>) -> Option<&str> {
        self.ends.end(n)
    }
}
