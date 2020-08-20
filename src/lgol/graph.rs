#![allow(unused_parens)]

use ars_aa::lattice::LatticeCanonicalizable;
use ars_aa::lattice::LatticeCanonicalizer;
use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;
use ars_ds::scalar::UScalar;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::bfs;
use crate::chunk_store;
use crate::dfs;
use crate::gol;
use crate::lgol;

use bfs::bfs2::Bfs2Dedupe;
use chunk_store::MmapChunkSafe;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use gol::printbag::PrintBag;
use lgol::axis::LGolAxis;
use lgol::axis::LGolEdgeRead;
use lgol::axis::LGolRecenter;
use lgol::axis::LGolRecenterCentered;
use lgol::axis::LGolRecenterJustify;
use lgol::bg::LGolBgCoord;
use lgol::constraints::LGolConstraint;
use lgol::ends::LGolEnds;
use lgol::lat1::LGolLat1;
use lgol::lat1::Vec3;
use lgol::lat2::LGolLat2;
use lgol::lat2::LGolShiftData;

pub trait RowTuple: Copy + Debug + Default + Eq + Hash + Send + Sync {
    type Item: UScalar;

    fn len() -> usize;
    fn as_slice(&self) -> &[Self::Item];
    fn as_slice_mut(&mut self) -> &mut [Self::Item];
    fn from_slice(slice: &[Self::Item]) -> Self;
    fn mask(&mut self, other: &Self);
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

            fn mask(&mut self, other: &[B; $n]) {
                for (r, m) in self.iter_mut().zip(other.iter()) {
                    *r &= *m;
                }
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
#[derive(Copy)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
#[derive(Serialize)]
pub struct LGolNode<BS: RowTuple, BC, CSS> {
    pub bg_coord: BC,
    pub du: i16,
    pub dv: i16,
    pub r0s: BS,
    pub r1: BS::Item,
    pub r1l: u8,
    pub r1_css: CSS,
}

impl<BS: RowTuple, BC: Nice + Default, CSS: Nice + Default> MmapChunkSafe for LGolNode<BS, BC, CSS> {
    // :X
}

impl<BS: RowTuple, BC: Nice, CSS: Nice> DfsNode for LGolNode<BS, BC, CSS> {
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

impl<BS: RowTuple, BC: Nice, CSS: Nice> LGolNode<BS, BC, CSS> {
    fn read_mask(&self, (idx, mask): (usize, BS::Item)) -> u32 {
        let r = match idx {
            0 => self.r1,
            _ => self.r0s.as_slice()[idx - 1],
        };
        (r & mask).count_ones()
    }
}

#[derive(Clone)]
#[derive(Copy)]
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

impl<BS: RowTuple, BC: Nice + Copy> MmapChunkSafe for LGolKeyNode<BS, BC> {
    // :X
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
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
#[derive(Serialize)]
pub struct LGolHashNode<BS: RowTuple, BC> {
    pub bg_coord: BC,
    pub rs: BS,
}

#[derive(Clone)]
pub struct LGolGraphParams<BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, CS: LGolConstraint<BC>> {
    pub vu: Vec3,
    pub vv: Vec3,
    pub vw: Vec3,

    pub bg_coord: PhantomData<BC>,

    pub u_axis: UA,
    pub v_axis: VA,
    pub constraints: CS,
}

enum PartialRowRead {
    Known(bool),
    Unknown,
    Read(usize, usize),
}

fn update_for_axis<BC: LGolBgCoord>(adet: isize, bg_coord: BC, a: &impl LGolAxis<BC>, shift_data: &LGolShiftData<BC>, c: &mut isize) -> Option<PartialRowRead> {
    let e;
    if *c < 0 {
        e = a.left_edge(shift_data, bg_coord, *c);
    }
    else if *c >= adet {
        e = a.right_edge(shift_data, bg_coord, *c);
    }
    else {
        return None;
    }
    match e {
        LGolEdgeRead::Known(b) => {
            return Some(PartialRowRead::Known(b));
        }
        LGolEdgeRead::Update(new_c) => {
            *c = new_c;
        }
        LGolEdgeRead::Wrap => {
            *c = (*c).rem_euclid(adet);
        }
        LGolEdgeRead::Unknown => {
            return Some(PartialRowRead::Unknown);
        }
    }
    None
}

impl<BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, CS: LGolConstraint<BC>> LGolGraphParams<BC, UA, VA, CS> {
    pub fn derived<BS: RowTuple, E: LGolEnds<BS, BC>>(&self, ends: E) -> LGolGraph<BS, BC, UA, VA, CS, E> {
        let lat1 = LGolLat1::new(self.vu, self.vv, self.vw);
        let lat2 = LGolLat2::new(&lat1);

        let xyt_idx = lat2.spots.iter().enumerate().map(|(idx, &(xyt, _uvw, _))| (xyt, idx)).collect::<HashMap<_, _>>();

        let compute_prow_read = |bg_coord: BC, rl, xyt| {
            let bg_coord = bg_coord.add(BC::from_xyt(xyt));
            let (mut u, mut v, w) = lat1.xyt_to_uvw(xyt);

            if let Some(r) = update_for_axis(lat1.adet, bg_coord, &self.v_axis, &lat2.v_shift_data, &mut v) {
                return r;
            }
            if let Some(r) = update_for_axis(lat1.adet, bg_coord, &self.u_axis, &lat2.u_shift_data, &mut u) {
                return r;
            }

            let lw = w.div_euclid(lat1.adet);
            let w = w.rem_euclid(lat1.adet);

            let xyt = lat1.uvw_to_xyt((u, v, w));
            let idx = *xyt_idx.get(&xyt).unwrap();

            if lw > 0 {
                // not yet building
                return PartialRowRead::Unknown
            }
            if lw < 0 {
                // already built, will necessarily be able to read
                return PartialRowRead::Read((-lw) as usize, idx)
            }

            // in the current row, see if it will already be set
            if idx < rl {
                return PartialRowRead::Read(0, idx);
            }
            else {
                return PartialRowRead::Unknown;
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

        let checks: Vec<_> = (0..BC::max_idx()).map(|bg_idx| {
            let bg_coord = BC::from_idx(bg_idx);
            let checks: Vec<_> = lat2.spots.iter().enumerate().map(|(idx, &(xyt, _, _))| compute_checks(bg_coord, idx, xyt)).collect();

            checks
        }).collect();

        let max_row_idx = checks.iter().map(|checks| {
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

pub struct LGolGraph<BS: RowTuple, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, CS: LGolConstraint<BC>, E: LGolEnds<BS, BC>> {
    pub params: LGolGraphParams<BC, UA, VA, CS>,
    pub lat1: LGolLat1,
    pub lat2: LGolLat2<BC>,

    pub max_r1l: usize,
    pub checks: Vec<Vec<Vec<(Vec<(usize, BS::Item)>, u32, u32, (usize, BS::Item, bool), (usize, BS::Item, bool))>>>,

    pub ends: E,

    _bs: PhantomData<BS>,
}

impl<BS: RowTuple, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, CS: LGolConstraint<BC>, E: LGolEnds<BS, BC>> LGolGraph<BS, BC, UA, VA, CS, E> {
    fn recenter_common(&self, ty: impl LGolRecenter, hn: LGolHashNode<BS, BC>) -> (isize, isize, LGolHashNode<BS, BC>) {
        let (sv, hn) = ty.apply(&self.params.v_axis, &self.lat2.v_shift_data, hn);
        let (su, hn) = ty.apply(&self.params.u_axis, &self.lat2.u_shift_data, hn);

        let du = su * self.lat2.u_shift_data.period;
        let dv = sv * self.lat2.v_shift_data.period;
        (du, dv, hn)
    }

    fn recenter(&self, hn: LGolHashNode<BS, BC>) -> (isize, isize, LGolHashNode<BS, BC>) {
        self.recenter_common(LGolRecenterCentered(), hn)
    }

    fn justify(&self, hn: LGolHashNode<BS, BC>) -> (isize, isize, LGolHashNode<BS, BC>) {
        self.recenter_common(LGolRecenterJustify(), hn)
    }

    fn expand_srch(&self, n1: &LGolNode<BS, BC, CS::S>, n2s: &mut Vec<LGolNode<BS, BC, CS::S>>) {
        let idx = n1.r1l as usize;

        if idx == self.max_r1l {
            let mut r0s = BS::default();

            r0s.as_slice_mut()[1..BS::len()].copy_from_slice(&n1.r0s.as_slice()[0..(BS::len() - 1)]);
            r0s.as_slice_mut()[0] = n1.r1;
            let bg_coord = n1.bg_coord.add(self.lat2.w_bg_coord);

            let hn = LGolHashNode {
                bg_coord: bg_coord,
                rs: r0s,
            };
            let (du, dv, hn) = self.recenter(hn);

            // TODO: this check is wrong for double BG
            if n1.r0s == BS::default() && (du, dv) != (0, 0) {
                // reject stupid first row shifts
                return;
            }

            n2s.push(LGolNode {
                bg_coord: hn.bg_coord,
                du: n1.du + (du as i16),
                dv: n1.dv + (dv as i16),
                r0s: hn.rs,
                r1: BS::Item::zero(),
                r1_css: self.params.constraints.zero_stat(self),
                r1l: 0,
            });
            return;
        }

        let (_, _, idx_bg_coord) = self.lat2.spots[idx];
        let v_bg_coord = n1.bg_coord.add(idx_bg_coord);

        'v: for &v in &[false, true] {
            let mut r1 = n1.r1;
            r1.set_bit(idx, v);

            let n2 = LGolNode {
                bg_coord: n1.bg_coord,
                du: n1.du,
                dv: n1.dv,
                r0s: n1.r0s,
                r1: r1,
                r1l: n1.r1l + 1,
                r1_css: match self.params.constraints.add_stat(self, n1.r1_css, v_bg_coord, r1, idx, v) {
                    Some(css) => css,
                    None => {
                        continue 'v;
                    }
                },
            };

            let checks = &self.checks[n2.bg_coord.to_idx()];
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
            let (ux, uy, ut) = self.params.vu;
            wraps.push((uy, ux, ut));
        }
        if self.params.v_axis.wrap_in_print() {
            let (vx, vy, vt) = self.params.vv;
            wraps.push((vy, vx, vt));
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
            let (y, x, t) = wraps.canonicalize((y, x, t));
            pr.insert(x, y, t, c);
        }
    }

    pub fn format_rows(&self, rows: &Vec<LGolKeyNode<BS, BC>>, last: Option<&LGolNode<BS, BC, CS::S>>) -> Vec<String> {
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

    pub fn format_cycle_rows_hack(&self, cycle: &Vec<LGolKeyNode<BS, BC>>) -> Vec<String> {
        let mut pr = PrintBag::new();
        let mut w = 0;
        let last = BS::len() - 1;
        for row in cycle.iter() {
            self.collect_row(&mut pr, '.', '*', row.rs.as_slice()[last], None, (row.du as isize), (row.dv as isize), w);
            w += 1;
        }
        pr.format()
    }

    pub fn zero_node(&self) -> LGolNode<BS, BC, CS::S> {
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

    #[allow(dead_code)]
    pub fn parse_bs2<S: AsRef<str>>(&self, rs: impl IntoIterator<Item=S>) -> BS {
        let mut bag = HashMap::new();
        for (y, line) in rs.into_iter().enumerate() {
            let line = line.as_ref();

            let mut t = 0;
            let mut x = 0;
            for c in line.chars() {
                match c {
                    '|' => {
                        t += 1;
                        x = 0;
                    },
                    ' ' => {
                        x += 1;
                    },
                    _ => {
                        let xyt = (x, y as isize, t);
                        let already = bag.insert(xyt, c);
                        assert_eq!(already, None, "collision at {:?}", xyt);
                        x += 1;
                    },
                }
            }
        }
        self.parse_bs2_bag(&bag)
    }

    pub fn parse_bs2_bag(&self, bag0: &HashMap<Vec3, char>) -> BS {
        let mut wraps = vec![];
        if self.params.u_axis.wrap_in_print() {
            wraps.push(self.params.vu);
        }
        if self.params.v_axis.wrap_in_print() {
            wraps.push(self.params.vv);
        }
        let wraps = Vec3::canonicalize(wraps);

        let mut bag = HashMap::new();
        for (&xyt, &c) in bag0.iter() {
            let xyt = wraps.canonicalize(xyt);
            let already = bag.insert(xyt, c);
            assert_eq!(already, None, "collision at {:?}", xyt);
        }
        let (&base, _) = bag.iter().filter(|&(_, &c)| c == 'z').next().unwrap();
        bag.remove(&base);

        let mut r0s = BS::default();

        let (ix, iy, it) = base;
        let (wx, wy, wt) = self.lat1.w_to_xyt;

        for (idx, &((sx, sy, st), _uvw, _bg_coord)) in self.lat2.spots.iter().enumerate() {
            for (j, r0) in r0s.as_slice_mut().iter_mut().enumerate() {
                let x = ix + sx - ((j as isize) + 1) * wx;
                let y = iy + sy - ((j as isize) + 1) * wy;
                let t = it + st - ((j as isize) + 1) * wt;

                let xyt = (x, y, t);
                let xyt = wraps.canonicalize(xyt);

                match bag.remove(&xyt) {
                    Some('*') => r0.set_bit(idx, true),
                    Some('.') => {
                    },
                    None => {
                    }
                    _ => panic!(),
                }
            }
        }

        assert_eq!(bag, HashMap::new());

        r0s
    }

    pub fn recenter_xyt(&self, xyt: Vec3, rs: BS) -> (Vec3, BS) {
        let hn = LGolHashNode {
            bg_coord: BC::from_xyt(xyt),
            rs: rs,
        };
        let (du, dv, hn) = self.recenter(hn);
        let (x, y, t) = xyt;
        let (dx, dy, dt) = self.lat1.uvw_to_xyt((du, dv, 0));
        ((x + dx, y + dy, t + dt), hn.rs)
    }

    pub fn regular_node(&self, xyt: Vec3, r0s: BS) -> LGolNode<BS, BC, CS::S> {
        let (u, v, _w) = self.lat1.xyt_to_uvw(xyt);

        // Mmm, sort of complicated.  Our conversion to uvw definitely doesn't include this (since
        // we only have du and dv in node), but we get bg_coord right.  Without the assert we're
        // wrong in two possible ways: (a) display will be wrong since it converts back w/o w and
        // (b) display could actually crash if (u, v, 0) can't be converted back to integral xyt.
        //
        // We skip this and hope our callers will be okay...
        //assert_eq!(w, 0);

        LGolNode {
            bg_coord: BC::from_xyt(xyt),
            du: (u as i16),
            dv: (v as i16),
            r0s: r0s,
            r1: BS::Item::zero(),
            r1l: 0,
            r1_css: self.params.constraints.zero_stat(self),
        }
    }

    #[allow(dead_code)]
    pub fn cb_node(&self, xyt0: Vec3, mut f: impl FnMut(Vec3) -> bool) -> LGolNode<BS, BC, CS::S> {
        let mut r0s = BS::default();

        let (ix, iy, it) = xyt0;
        let (wx, wy, wt) = self.lat1.w_to_xyt;

        for (idx, &((sx, sy, st), _uvw, _bg_coord)) in self.lat2.spots.iter().enumerate() {
            for (j, r0) in r0s.as_slice_mut().iter_mut().enumerate() {
                let x = ix + sx - ((j as isize) + 1) * wx;
                let y = iy + sy - ((j as isize) + 1) * wy;
                let t = it + st - ((j as isize) + 1) * wt;

                if f((x, y, t)) {
                    r0.set_bit(idx, true);
                }
            }
        }

        self.regular_node(xyt0, r0s)
    }

    pub fn key_node(&self, xyt: Vec3, rs: BS) -> LGolKeyNode<BS, BC> {
        let (u, v, _w) = self.lat1.xyt_to_uvw(xyt);

        // Mmm, sort of complicated.  Our conversion to uvw definitely doesn't include this (since
        // we only have du and dv in node), but we get bg_coord right.  Without the assert we're
        // wrong in two possible ways: (a) display will be wrong since it converts back w/o w and
        // (b) display could actually crash if (u, v, 0) can't be converted back to integral xyt.
        //
        // We skip this and hope our callers will be okay...
        //assert_eq!(w, 0);

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

impl<BS: RowTuple, BC: LGolBgCoord, UA: LGolAxis<BC>, VA: LGolAxis<BC>, CS: LGolConstraint<BC>, E: LGolEnds<BS, BC>> DfsGraph<LGolNode<BS, BC, CS::S>> for LGolGraph<BS, BC, UA, VA, CS, E> {
    fn expand(&self, n1: &LGolNode<BS, BC, CS::S>) -> Vec<LGolNode<BS, BC, CS::S>> {
        let mut n2s = Vec::new();
        self.expand_srch(n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &LGolKeyNode<BS, BC>) -> Option<&str> {
        let mut hn = n.lgol_hash_node();
        if self.ends.want_justify() {
            hn = self.justify(hn).2;
        }
        self.ends.end(&hn)
    }
}

pub struct LGolDedupeHack<BS: RowTuple>(Vec<HashSet<BS>>);

impl<BS: RowTuple, BC: LGolBgCoord, CSS: Nice, CF> Bfs2Dedupe<LGolNode<BS, BC, CSS>, CF> for LGolDedupeHack<BS> {
    fn new(_cf: CF) -> Self {
        LGolDedupeHack((0..BC::max_idx()).map(|_| HashSet::new()).collect())
    }

    fn len(&self) -> usize {
        self.0.iter().map(|s| s.len()).sum()
    }

    fn cloned_iter<'a>(&'a self) -> Box<dyn Iterator<Item=LGolHashNode<BS, BC>> + 'a> {
        Box::new(self.0.iter().enumerate().flat_map(|(i, s)| {
            s.iter().map(move |rs| {
                LGolHashNode {
                    bg_coord: BC::from_idx(i),
                    rs: rs.clone(),
                }
            })
        }))
    }

    fn insert(&mut self, n: LGolHashNode<BS, BC>) -> bool {
        self.0[n.bg_coord.to_idx()].insert(n.rs)
    }
}
