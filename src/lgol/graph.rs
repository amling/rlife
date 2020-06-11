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
    fn get(&self, idx: usize) -> Self::Item;
    fn set(&mut self, idx: usize, v: Self::Item);
}

macro_rules! impl_row_tuple {
    ($n:expr) => {
        impl<B: UScalar> RowTuple for [B; $n] {
            type Item = B;

            fn len() -> usize {
                $n
            }

            fn get(&self, idx: usize) -> B {
                self[idx]
            }

            fn set(&mut self, idx: usize, v: B) {
                self[idx] = v;
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
            rs: self.r0s,
        })
    }
}

impl<BS: RowTuple, US: LGolAxisStats, VS: LGolAxisStats> LGolNode<BS, US, VS> {
    fn read_mask(&self, (idx, mask): (usize, BS::Item)) -> u32 {
        let r = match idx {
            0 => self.r1,
            _ => self.r0s.get(idx - 1),
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

    fn zero_stat(&self) -> Self::S;
    fn add_stat(&self, s0: Self::S, v: isize, c: bool) -> Option<Self::S>;

    fn recenter<BS: RowTuple>(&self, rs: BS) -> Option<BS>;

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

    fn zero_stat(&self) {
    }

    fn add_stat(&self, _s0: (), _v: isize, _c: bool) -> Option<()> {
        Some(())
    }

    fn recenter<BS: RowTuple>(&self, rs: BS) -> Option<BS> {
        Some(rs)
    }

    fn wrap_in_print(&self) -> bool {
        self == &(LGolEdge::Wrap, LGolEdge::Wrap)
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

        let compute_shift_rows = |mangle: &dyn Fn(Vec3) -> Vec3| {
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

            LGolShiftRows {
                period: period,
                shift_rows: shift_rows,
            }
        };

        let u_shift_rows = compute_shift_rows(&|uvw| uvw);
        let v_shift_rows = compute_shift_rows(&|(u, v, w)| (v, u, w));

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

        LGolGraph {
            params: self.clone(),

            spots: spots,
            max_r1l: lc.adet as usize,
            checks: checks,
            u_shift_rows: u_shift_rows,
            v_shift_rows: v_shift_rows,

            _bs: PhantomData::default(),
        }
    }
}

pub struct LGolShiftRows {
    period: isize,
    shift_rows: Vec<Vec<usize>>,
}

pub struct LGolGraph<BS: RowTuple, UA: LGolAxis, VA: LGolAxis> {
    pub params: LGolGraphParams<UA, VA>,

    pub spots: Vec<(Vec3, Vec3)>,
    pub max_r1l: usize,
    pub checks: Vec<Vec<(Vec<(usize, BS::Item)>, u32, (usize, BS::Item), (usize, BS::Item))>>,
    pub u_shift_rows: LGolShiftRows,
    pub v_shift_rows: LGolShiftRows,

    _bs: PhantomData<BS>,
}

impl<BS: RowTuple, UA: LGolAxis, VA: LGolAxis> LGolGraph<BS, UA, VA> {
    fn recenter(&self, rs: BS) -> BS {
        return rs;
    }

    fn expand_srch(&self, n1: &LGolNode<BS, UA::S, VA::S>, n2s: &mut Vec<LGolNode<BS, UA::S, VA::S>>) {
        let idx = n1.r1l as usize;

        if idx == self.max_r1l {
            let mut r0s = BS::default();

            for i in (1..BS::len()) {
                r0s.set(i, n1.r0s.get(i - 1));
            }
            r0s.set(0, n1.r1);

            let r0s = match self.params.u_axis.recenter(r0s) {
                Some(r0s) => r0s,
                None => {
                    return;
                }
            };
            let r0s = match self.params.v_axis.recenter(r0s) {
                Some(r0s) => r0s,
                None => {
                    return;
                }
            };

            n2s.push(LGolNode {
                r0s: r0s,
                r1: BS::Item::zero(),
                r1_us: self.params.u_axis.zero_stat(),
                r1_vs: self.params.v_axis.zero_stat(),
                r1l: 0,
            });
            return;
        }

        'v: for &v in &[false, true] {
            let (idx_u, idx_v, _) = self.spots[idx].1;
            let mut n2 = LGolNode {
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

    fn collect_row(&self, pr: &mut PrintBag, c0: char, c1: char, row: BS::Item, rl: Option<usize>, w: isize) {
        let mut wraps = vec![];
        if self.params.u_axis.wrap_in_print() {
            wraps.push(self.params.vu);
        }
        if self.params.v_axis.wrap_in_print() {
            wraps.push(self.params.vv);
        }
        let wraps = Vec3::canonicalize(wraps);

        for (idx, &((x, y, t), _uvw)) in self.spots.iter().enumerate() {
            let (wx, wy, wt) = self.params.vw;

            let x = x + w * wx;
            let y = y + w * wy;
            let t = t + w * wt;

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
            if n == 0 {
                for i in (1..BS::len()).rev() {
                    self.collect_row(&mut pr, '.', '*', row.rs.get(i), None, w);
                    w += 1;
                }
            }
            self.collect_row(&mut pr, '.', '*', row.rs.get(0), None, w);
            w += 1;
        }
        if let Some(last) = last {
            self.collect_row(&mut pr, '.', '*', last.r1, Some(last.r1l as usize), w);
        }
        pr.format()
    }

    pub fn format_cycle_rows(&self, path: &Vec<LGolKeyNode<BS>>, cycle: &Vec<LGolKeyNode<BS>>, _last: &LGolKeyNode<BS>) -> Vec<String> {
        let mut pr = PrintBag::new();
        let mut w = 0;
        let last = BS::len() - 1;
        for row in path.iter() {
            self.collect_row(&mut pr, '.', '*', row.rs.get(last), None, w);
            w += 1;
        }
        for row in cycle.iter() {
            self.collect_row(&mut pr, 'x', 'o', row.rs.get(last), None, w);
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
        for i in 0..BS::len() {
            if n.rs.get(i) != BS::Item::zero() {
                return None;
            }
        }
        Some("")
    }
}
