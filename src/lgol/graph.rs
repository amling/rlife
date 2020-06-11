#![allow(unused_parens)]

use ars_aa::lattice::LatticeCanonicalizable;
use ars_aa::lattice::LatticeCanonicalizer;
use ars_ds::scalar::Scalar;
use ars_ds::scalar::UScalar;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
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
use lgol::lattice::LatticeCoords;
use lgol::lattice::Vec3;

marker_trait! {
    LGolAxisStats:
    [Copy]
    [Debug]
    [Default]
    [Eq]
    [Hash]
    [Send]
    [Sync]
}

pub trait RowTuple: Copy + Debug + Default + Eq + Hash + Send + Sync {
    type Item: UScalar;

    fn len() -> usize;
    fn as_slice(&self) -> &[Self::Item];
    fn as_slice_mut(&mut self) -> &mut [Self::Item];
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
        }
    }
}

impl_row_tuple!(1);
impl_row_tuple!(2);
impl_row_tuple!(3);
impl_row_tuple!(4);
impl_row_tuple!(5);
impl_row_tuple!(6);

#[derive(Clone)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
#[derive(Serialize)]
pub struct LGolNode<BS: RowTuple, US, VS> {
    pub du: i16,
    pub dv: i16,
    pub r0s: BS,
    pub r1: BS::Item,
    pub r1l: u8,
    pub r1_us: US,
    pub r1_vs: VS,
}

impl<BS: RowTuple, US: LGolAxisStats, VS: LGolAxisStats> DfsNode for LGolNode<BS, US, VS> {
    type KN = LGolKeyNode<BS>;

    fn key_node(&self) -> Option<LGolKeyNode<BS>> {
        if self.r1l != 0 {
            return None;
        }

        Some(LGolKeyNode {
            du: self.du,
            dv: self.dv,
            rs: self.r0s,
        })
    }
}

impl<BS: RowTuple, US: LGolAxisStats, VS: LGolAxisStats> LGolNode<BS, US, VS> {
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
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct LGolKeyNode<BS: RowTuple> {
    pub du: i16,
    pub dv: i16,
    pub rs: BS,
}

impl<BS: RowTuple> DfsKeyNode for LGolKeyNode<BS> {
    type HN = LGolHashNode<BS>;

    fn hash_node<'a>(&'a self, _path: impl Iterator<Item=&'a LGolKeyNode<BS>>) -> Option<LGolHashNode<BS>> {
        Some(LGolHashNode {
            rs: self.rs,
        })
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Default)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct LGolHashNode<BS: RowTuple> {
    pub rs: BS,
}

#[derive(Clone)]
#[derive(Copy)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(PartialEq)]
pub enum LGolEdge {
    Empty,
    Wrap,
    Unknown,
}

pub trait LGolAxis: Copy {
    type S: LGolAxisStats;

    fn left_edge(&self) -> LGolEdge;
    fn right_edge(&self) -> LGolEdge;

    fn zero_stat(&self, shift_data: &LGolShiftData) -> Self::S;
    fn add_stat(&self, s0: Self::S, c: isize, v: bool) -> Option<Self::S>;

    fn recenter<BS: RowTuple>(&self, shift_data: &LGolShiftData, rs: BS) -> (isize, BS);

    fn wrap_in_print(&self) -> bool;
}

impl LGolAxis for (LGolEdge, LGolEdge) {
    type S = ();

    fn left_edge(&self) -> LGolEdge {
        self.0
    }

    fn right_edge(&self) -> LGolEdge {
        self.1
    }

    fn zero_stat(&self, _shift_data: &LGolShiftData) {
    }

    fn add_stat(&self, _s0: (), _c: isize, _v: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, _shift_data: &LGolShiftData, rs: BS) -> (isize, BS) {
        (0, rs)
    }

    fn wrap_in_print(&self) -> bool {
        self == &(LGolEdge::Wrap, LGolEdge::Wrap)
    }
}

#[derive(Clone)]
#[derive(Copy)]
pub struct LGolFancyAxis {
    // TODO: determinant denominator issues (check below assumes this is in units of 1/|det| but user doesn't want to have to care about det)
    w: isize,
}

impl LGolAxis for LGolFancyAxis {
    type S = (isize, isize);

    fn left_edge(&self) -> LGolEdge {
        LGolEdge::Empty
    }

    fn right_edge(&self) -> LGolEdge {
        LGolEdge::Empty
    }

    fn zero_stat(&self, shift_data: &LGolShiftData) -> (isize, isize) {
        (shift_data.max_coord, shift_data.min_coord)
    }

    fn add_stat(&self, s0: (isize, isize), c: isize, v: bool) -> Option<(isize, isize)> {
        if !v {
            return Some(s0);
        }

        let min = s0.0.min(c);
        let max = s0.1.max(c);

        if max >= min + self.w {
            return None;
        }

        Some((min, max))
    }

    fn recenter<BS: RowTuple>(&self, shift_data: &LGolShiftData, rs: BS) -> (isize, BS) {
        let min = shift_data.find_bs_min(rs);
        let max = shift_data.find_bs_max(rs);

        let our_sum = min + max;
        let def_sum = shift_data.min_coord + shift_data.max_coord;

        let delta = our_sum - def_sum;
        let delta = delta.div_euclid(2 * shift_data.period);

        if delta == 0 {
            return (0, rs);
        }

        let dc = delta * shift_data.period;

        let mut rss = BS::default();
        for shift_row in shift_data.shift_rows.iter() {
            for (i, &idx) in shift_row.iter().enumerate() {
                for j in 0..BS::len() {
                    if rs.as_slice()[j].get_bit(idx) {
                        // don't compute this earlier as it may be OOB
                        let i2 = (((i as isize) - delta) as usize);
                        let idx2 = shift_row[i2];
                        rss.as_slice_mut()[j].set_bit(idx2, true);
                    }
                }
            }
        }

        (dc, rss)
    }

    fn wrap_in_print(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct LGolGraphParams<UA: LGolAxis, VA: LGolAxis> {
    pub vu: Vec3,
    pub vv: Vec3,
    pub vw: Vec3,

    pub u_axis: UA,
    pub v_axis: VA,
}

enum PartialRowRead {
    Off,
    Unknown,
    Read(usize, usize),
}

impl<UA: LGolAxis, VA: LGolAxis> LGolGraphParams<UA, VA> {
    pub fn derived<BS: RowTuple>(&self) -> LGolGraph<BS, UA, VA> {
        let lc = LatticeCoords::new(self.vu, self.vv, self.vw);

        // step two: figure out (x, y, t) coordinates for fundamental volume
        let mut spots = Vec::new();
        for t in 0..lc.mt {
            for x in 0..lc.mx {
                for y in 0..lc.my {
                    // this (x, y, t) is some equivalence class, but we want to shift it to be in
                    // [0, 1)x[0, 1)x[0, 1) in uvw space
                    let (xyt, _) = lc.canonicalize_xyt((x, y, t));

                    let uvw = lc.xyt_to_uvw(xyt);

                    spots.push((xyt, uvw));
                }
            }
        }

        spots.sort_by_key(|&(_, (u, v, _w))| {
            // Ugh, in order to get u and v handled sanely in the "single w layer" case we have to
            // ignore w (since some of them are shifted up in w space).  This will likely need
            // revisitting when/if we do any "multiple w layer" searches...
            (u, v)
        });

        let compute_shift_data = |mangle: &dyn Fn(Vec3) -> Vec3| {
            let period = {
                let v1 = mangle(lc.xyt_to_uvw((1, 0, 0)));
                let v2 = mangle(lc.xyt_to_uvw((0, 1, 0)));
                let v3 = mangle(lc.xyt_to_uvw((0, 0, 1)));

                let l3 = Vec3::canonicalize(vec![v1, v2, v3]);
                let (_, (_, (lc, ()))) = l3;
                lc.unwrap().0
            };

            let mut row_c_idx = BTreeMap::new();
            for (idx, &(_xyt, uvw)) in spots.iter().enumerate() {
                let (c, other, w) = mangle(uvw);
                row_c_idx.entry((other, w)).or_insert_with(|| BTreeMap::new()).insert(c, idx);
            }

            let shift_rows: Vec<Vec<_>> = row_c_idx.into_iter().map(|(_row, c_idx)| {
                let c_idx: Vec<_> = c_idx.into_iter().collect();
                for i in 0..(c_idx.len() - 1) {
                    assert_eq!(c_idx[i].0 + period, c_idx[i + 1].0);
                }
                c_idx.into_iter().map(|(_c, idx)| idx).collect()
            }).collect();

            let mut coord_idx_sorted: Vec<_> = spots.iter().enumerate().map(|(idx, &(_xyt, uvw))| (mangle(uvw).0, idx)).collect();
            coord_idx_sorted.sort();
            let min_coord = spots.iter().map(|&(_xyt, uvw)| mangle(uvw).0).min().unwrap();
            let max_coord = spots.iter().map(|&(_xyt, uvw)| mangle(uvw).0).max().unwrap();

            LGolShiftData {
                period: period,
                shift_rows: shift_rows,
                coord_idx_sorted: coord_idx_sorted,
                min_coord: min_coord,
                max_coord: max_coord,
            }
        };

        let u_shift_data = compute_shift_data(&|uvw| uvw);
        let v_shift_data = compute_shift_data(&|(u, v, w)| (v, u, w));

        let xyt_idx = spots.iter().enumerate().map(|(idx, &(xyt, _uvw))| (xyt, idx)).collect::<HashMap<_, _>>();

        let compute_prow_read = |rl, (x, y, t)| {
            let (xyt, (lu, lv, lw)) = lc.canonicalize_xyt((x, y, t));

            let u_edge;
            if lu < 0 {
                u_edge = self.u_axis.left_edge();
            }
            else if lu > 0 {
                u_edge = self.u_axis.right_edge();
            }
            else {
                u_edge = LGolEdge::Wrap;
            }

            let v_edge;
            if lv < 0 {
                v_edge = self.v_axis.left_edge();
            }
            else if lv > 0 {
                v_edge = self.v_axis.right_edge();
            }
            else {
                v_edge = LGolEdge::Wrap;
            }

            let edge = match u_edge {
                LGolEdge::Wrap => v_edge,
                _ => match v_edge {
                    LGolEdge::Wrap => u_edge,
                    _ => panic!("Edge conflict at {:?}, u_edge {:?}, v_edge {:?}", (x, y, t), u_edge, v_edge),
                }
            };

            match edge {
                LGolEdge::Empty => PartialRowRead::Off,
                LGolEdge::Wrap => {
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
                LGolEdge::Unknown => PartialRowRead::Unknown,
            }
        };

        let single_mask = |idx| {
            let mut b = BS::Item::zero();
            b.set_bit(idx, true);
            b
        };

        let compute_checks = |idx, (x, y, t)| {
            let rl = idx + 1;

            let mut checks = Vec::new();

            let mut f = |dx, dy, dt| {
                let x2 = x + dx;
                let y2 = y + dy;
                let t2 = t + dt;

                let cur_mask = match compute_prow_read(rl, (x2, y2, t2)) {
                    PartialRowRead::Off => (0, BS::Item::zero()),
                    PartialRowRead::Unknown => {
                        return;
                    },
                    PartialRowRead::Read(row_idx, bit_idx) => (row_idx, single_mask(bit_idx)),
                };
                let fut_mask = match compute_prow_read(rl, (x2, y2, t2 + 1)) {
                    PartialRowRead::Off => (0, BS::Item::zero()),
                    PartialRowRead::Unknown => {
                        return;
                    },
                    PartialRowRead::Read(row_idx, bit_idx) => (row_idx, single_mask(bit_idx)),
                };

                let mut nh_masks: Vec<(usize, BS::Item)> = Vec::new();
                let mut nh_ct = 0;

                for dx2 in -1..=1 {
                    'dy2: for dy2 in -1..=1 {
                        if (dx2, dy2) == (0, 0) {
                            continue;
                        }

                        let x3 = x2 + dx2;
                        let y3 = y2 + dy2;
                        let t3 = t2;

                        match compute_prow_read(rl, (x3, y3, t3)) {
                            PartialRowRead::Off => {
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

                checks.push((nh_masks, nh_ct, cur_mask, fut_mask));
            };

            for dx in -1..=1 {
                for dy in -1..=1 {
                    f(dx, dy, 0);
                }
            }
            f(0, 0, -1);

            checks
        };

        let checks: Vec<_> = spots.iter().enumerate().map(|(idx, &(xyt, _))| compute_checks(idx, xyt)).collect();

        let max_row_idx = checks.iter().map(|checks| {
            checks.iter().map(|&(ref nh_masks, _, _, _)| {
                nh_masks.iter().map(|&(row_idx, _)| row_idx)
            }).flatten()
        }).flatten().max().unwrap();
        assert!(max_row_idx <= BS::len(), "{} <= {}", max_row_idx, BS::len());

        let max_r1l = (lc.adet as usize);

        LGolGraph {
            params: self.clone(),

            lc: lc,
            spots: spots,
            max_r1l: max_r1l,
            checks: checks,
            u_shift_data: u_shift_data,
            v_shift_data: v_shift_data,

            _bs: PhantomData::default(),
        }
    }
}

pub struct LGolShiftData {
    period: isize,
    shift_rows: Vec<Vec<usize>>,
    coord_idx_sorted: Vec<(isize, usize)>,
    min_coord: isize,
    max_coord: isize,
}

impl LGolShiftData {
    fn find_bs_min<BS: RowTuple>(&self, rs: BS) -> isize {
        for &(c, idx) in self.coord_idx_sorted.iter() {
            for r in rs.as_slice() {
                if r.get_bit(idx) {
                    return c;
                }
            }
        }
        self.max_coord
    }

    fn find_bs_max<BS: RowTuple>(&self, rs: BS) -> isize {
        for &(c, idx) in self.coord_idx_sorted.iter().rev() {
            for r in rs.as_slice() {
                if r.get_bit(idx) {
                    return c;
                }
            }
        }
        self.min_coord
    }
}

pub struct LGolGraph<BS: RowTuple, UA: LGolAxis, VA: LGolAxis> {
    pub params: LGolGraphParams<UA, VA>,

    pub lc: LatticeCoords,
    pub spots: Vec<(Vec3, Vec3)>,
    pub max_r1l: usize,
    pub checks: Vec<Vec<(Vec<(usize, BS::Item)>, u32, (usize, BS::Item), (usize, BS::Item))>>,
    pub u_shift_data: LGolShiftData,
    pub v_shift_data: LGolShiftData,

    _bs: PhantomData<BS>,
}

impl<BS: RowTuple, UA: LGolAxis, VA: LGolAxis> LGolGraph<BS, UA, VA> {
    fn recenter(&self, rs: BS) -> (isize, isize, BS) {
        let (du, rs) = self.params.u_axis.recenter(&self.u_shift_data, rs);
        let (dv, rs) = self.params.v_axis.recenter(&self.v_shift_data, rs);
        (du, dv, rs)
    }

    fn expand_srch(&self, n1: &LGolNode<BS, UA::S, VA::S>, n2s: &mut Vec<LGolNode<BS, UA::S, VA::S>>) {
        let idx = n1.r1l as usize;

        if idx == self.max_r1l {
            let mut r0s = BS::default();

            r0s.as_slice_mut()[1..BS::len()].copy_from_slice(&n1.r0s.as_slice()[0..(BS::len() - 1)]);
            r0s.as_slice_mut()[0] = n1.r1;

            let (du, dv, r0s) = self.recenter(r0s);

            if n1.r0s == BS::default() && (du, dv) != (0, 0) {
                // reject stupid first row shifts
                return;
            }

            n2s.push(LGolNode {
                du: n1.du + (du as i16),
                dv: n1.dv + (dv as i16),
                r0s: r0s,
                r1: BS::Item::zero(),
                r1_us: self.params.u_axis.zero_stat(&self.u_shift_data),
                r1_vs: self.params.v_axis.zero_stat(&self.v_shift_data),
                r1l: 0,
            });
            return;
        }

        'v: for &v in &[false, true] {
            let (idx_u, idx_v, _) = self.spots[idx].1;
            let mut n2 = LGolNode {
                du: n1.du,
                dv: n1.dv,
                r0s: n1.r0s,
                r1: n1.r1,
                r1l: n1.r1l + 1,
                r1_us: match self.params.u_axis.add_stat(n1.r1_us, idx_u, v) {
                    Some(us) => us,
                    None => {
                        continue 'v;
                    }
                },
                r1_vs: match self.params.v_axis.add_stat(n1.r1_vs, idx_v, v) {
                    Some(us) => us,
                    None => {
                        continue 'v;
                    }
                },
            };

            n2.r1.set_bit(idx, v);

            for &(ref nh_masks, nh_ct, cur_mask, fut_mask) in self.checks[idx].iter() {
                let mut nh = 0;
                for &nh_mask in nh_masks {
                    nh += n2.read_mask(nh_mask);
                }

                let cur_cell = (n2.read_mask(cur_mask) != 0);
                let fut_cell = (n2.read_mask(fut_mask) != 0);

                if !check_compat2(nh, nh_ct, cur_cell, fut_cell) {
                    continue 'v;
                }
            }

            n2s.push(n2.clone());
        }
    }

    fn collect_row(&self, pr: &mut PrintBag, c0: char, c1: char, row: BS::Item, rl: Option<usize>, du: isize, dv: isize, w: isize) {
        // awkwardly w is in actual steps of w while du and dv are in units of 1/|det|
        let (dx, dy, dt) = self.lc.uvw_to_xyt((du, dv, self.lc.adet * w));

        let mut wraps = vec![];
        if self.params.u_axis.wrap_in_print() {
            wraps.push(self.params.vu);
        }
        if self.params.v_axis.wrap_in_print() {
            wraps.push(self.params.vv);
        }
        let wraps = Vec3::canonicalize(wraps);

        for (idx, &((x, y, t), _uvw)) in self.spots.iter().enumerate() {
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

    pub fn format_rows(&self, rows: &Vec<LGolKeyNode<BS>>, last: Option<&LGolNode<BS, UA::S, VA::S>>) -> Vec<String> {
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

    pub fn format_cycle_rows(&self, path: &Vec<LGolKeyNode<BS>>, cycle: &Vec<LGolKeyNode<BS>>, _last: &LGolKeyNode<BS>) -> Vec<String> {
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

impl<BS: RowTuple, UA: LGolAxis, VA: LGolAxis> DfsGraph<LGolNode<BS, UA::S, VA::S>> for LGolGraph<BS, UA, VA> {
    fn expand<'a>(&'a self, n1: &'a LGolNode<BS, UA::S, VA::S>, _path: impl Iterator<Item=&'a LGolKeyNode<BS>>) -> Vec<LGolNode<BS, UA::S, VA::S>> {
        let mut n2s = Vec::new();
        self.expand_srch(n1, &mut n2s);
        n2s
    }

    fn end<'a>(&'a self, n: &'a LGolKeyNode<BS>, _path: impl Iterator<Item=&'a LGolKeyNode<BS>>) -> Option<&'static str> {
        for &r in n.rs.as_slice() {
            if r != BS::Item::zero() {
                return None;
            }
        }
        Some("")
    }
}
