#![allow(unused_parens)]

use ars_ds::scalar::UScalar;
use serde::Deserialize;
use serde::Serialize;

use crate::dfs;
use crate::gol;

use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use gol::printbag::PrintBag;

#[derive(Clone)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct GolNodeSerdeProxy<B: UScalar> {
    pub dx: isize,
    pub r0: B,
    pub r1: B,
    pub r2: B,
    pub r2l: usize,
}

impl<B: UScalar> GolNodeSerdeProxy<B> {
    pub fn to_real(&self, e: &GolGraph<B>) -> GolNode<B> {
        GolNode {
            dx: self.dx,
            r0: self.r0,
            r1: self.r1,
            r2: self.r2,
            r2_min_x: find_min_x(e, self.r2),
            r2_max_x: find_max_x(e, self.r2),
            r2l: self.r2l,
            r2l_x: (self.r2l % e.mx),
        }
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct GolNode<B: UScalar> {
    pub dx: isize,
    pub r0: B,
    pub r1: B,
    pub r2: B,
    pub r2_min_x: usize,
    pub r2_max_x: usize,
    pub r2l: usize,
    pub r2l_x: usize,
}

impl<B: UScalar> GolNode<B> {
    pub fn to_serde_proxy(&self, e: &GolGraph<B>) -> GolNodeSerdeProxy<B> {
        debug_assert_eq!(self.r2_min_x, find_min_x(e, self.r2));
        debug_assert_eq!(self.r2_max_x, find_max_x(e, self.r2));
        debug_assert_eq!(self.r2l_x, self.r2l % e.mx);

        GolNodeSerdeProxy {
            dx: self.dx,
            r0: self.r0,
            r1: self.r1,
            r2: self.r2,
            r2l: self.r2l,
        }
    }
}

impl<B: UScalar> DfsNode for GolNode<B> {
    type KN = GolKeyNode<B>;

    fn key_node(&self) -> Option<GolKeyNode<B>> {
        if self.r2l != 0 {
            return None;
        }

        Some(GolKeyNode {
            dx: self.dx,
            r0: self.r0,
            r1: self.r1,
        })
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct GolKeyNode<B: UScalar> {
    pub dx: isize,
    pub r0: B,
    pub r1: B,
}

impl<B: UScalar> DfsKeyNode for GolKeyNode<B> {
    type HN = GolHashNode<B>;

    fn hash_node(&self) -> GolHashNode<B> {
        GolHashNode {
            r0: self.r0,
            r1: self.r1,
        }
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct GolHashNode<B: UScalar> {
    pub r0: B,
    pub r1: B,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub enum GolRecenter {
    None,
    BiasLeft,
    BiasRight,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub enum GolSym {
    Empty,
    Odd,
    Even,
    Gutter,
    Wrap,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct GolPreGraph {
    pub mt: usize,
    pub mx: usize,
    pub wx: usize,

    pub left_sym: GolSym,
    pub right_sym: GolSym,

    pub ox: isize,
    pub oy: isize,

    pub recenter: GolRecenter,
}

enum PartialRowRead {
    Off,
    Unknown,
    Read(usize),
}

impl GolPreGraph {
    fn compute_shift(&self, t: usize, o: isize) -> isize {
        // cumulative shift after t steps can be floor(o * t / mt) so we diff cumulatives
        let t = t as isize;
        let mt = self.mt as isize;
        let before = (o * t) / mt;
        let after = (o * (t + 1)) / mt;
        after - before
    }

    fn compute_prow_read(&self, len: usize, t: usize, x: isize) -> PartialRowRead {
        let mx = self.mx as isize;
        let mut x = x;
        loop {
            if x < 0 {
                x = match self.left_sym {
                    GolSym::Empty => {
                        return PartialRowRead::Off;
                    },
                    GolSym::Odd => -x,
                    GolSym::Even => -x - 1,
                    GolSym::Gutter => {
                        if x == -1 {
                            return PartialRowRead::Off;
                        }
                        -x - 2
                    }
                    GolSym::Wrap => x + mx,
                };
                // reinterpret for something weird like e.g.  -2 wrapped to 2 in mx 1
                continue;
            }

            if x >= mx {
                x = match self.right_sym {
                    GolSym::Empty => {
                        return PartialRowRead::Off;
                    },
                    GolSym::Odd => 2 * mx - 2 - x,
                    GolSym::Even => 2 * mx - 1 - x,
                    GolSym::Gutter => {
                        if x == mx {
                            return PartialRowRead::Off;
                        }
                        2 * mx - x
                    },
                    GolSym::Wrap => x - mx,
                };
                // reinterpret
                continue;
            }

            let idx = (t * self.mx + (x as usize));
            if idx >= len {
                return PartialRowRead::Unknown;
            }

            return PartialRowRead::Read(idx);
        }
    }

    fn compute_checks2<B: UScalar>(&self, acc: &mut Vec<(Vec<(usize, B)>, u32, (usize, B), (usize, B))>, cp: (usize, usize), c: (usize, usize), cn: Option<(usize, usize)>, ct: usize, cx: isize, f: (usize, usize), ft: usize, fx: isize) {
        let single_mask = |idx| {
            let mut b = B::zero();
            b.set_bit(idx, true);
            b
        };

        let mut nh_masks: Vec<(usize, B)> = Vec::new();
        let mut nh_ct = 0;

        let mut add_nh = |r: (usize, usize), t, x| {
            match self.compute_prow_read(r.1, t, x) {
                PartialRowRead::Off => {
                    nh_ct += 1;
                }
                PartialRowRead::Unknown => {
                }
                PartialRowRead::Read(idx) => {
                    nh_ct += 1;

                    for &mut (nh_row_idx, ref mut nh_mask) in nh_masks.iter_mut() {
                        if nh_row_idx != r.0 {
                            continue;
                        }
                        if !nh_mask.get_bit(idx) {
                            nh_mask.set_bit(idx, true);
                            return;
                        }
                    }
                    nh_masks.push((r.0, single_mask(idx)));
                }
            }
        };

        add_nh(cp, ct, cx - 1);
        add_nh(cp, ct, cx);
        add_nh(cp, ct, cx + 1);
        add_nh(c, ct, cx - 1);
        add_nh(c, ct, cx + 1);
        if let Some(cn) = cn {
            add_nh(cn, ct, cx - 1);
            add_nh(cn, ct, cx);
            add_nh(cn, ct, cx + 1);
        }

        let cur_row_idx = c.0;
        let cur_mask = match self.compute_prow_read(c.1, ct, cx) {
            PartialRowRead::Off => B::zero(),
            PartialRowRead::Unknown => {
                return;
            }
            PartialRowRead::Read(idx) => single_mask(idx),
        };

        let fut_row_idx = f.0;
        let fut_mask = match self.compute_prow_read(f.1, ft, fx) {
            PartialRowRead::Off => B::zero(),
            PartialRowRead::Unknown => {
                return;
            }
            PartialRowRead::Read(idx) => single_mask(idx),
        };

        acc.push((nh_masks, nh_ct, (cur_row_idx, cur_mask), (fut_row_idx, fut_mask)));
    }

    fn compute_checks<B: UScalar>(&self, idx: usize) -> Vec<(Vec<(usize, B)>, u32, (usize, B), (usize, B))> {
        let x = idx % self.mx;
        let t = idx / self.mx;

        let ix = x as isize;

        // shift for the previous generation
        let pt = match t {
            0 => self.mt - 1,
            _ => t - 1,
        };
        let sxp = self.compute_shift(pt, self.ox);
        let syp = self.compute_shift(pt, self.oy);
        let px = ix - sxp;

        // shift from this time to the next
        let ft = match t == self.mt - 1 {
            true => 0,
            false => t + 1,
        };
        let sx = self.compute_shift(t, self.ox);
        let sy = self.compute_shift(t, self.oy);
        let fx = ix + sx;

        let mut acc = Vec::new();

        let r0 = (0, self.mx * self.mt);
        let r1 = (1, self.mx * self.mt);
        let r2 = (2, idx + 1);

        // check past cell if there is one (y shifts backwards!)
        match syp {
            1 => self.compute_checks2(&mut acc, r0, r1, Some(r2), pt, px, r2, t, ix),
            0 => self.compute_checks2(&mut acc, r1, r2, None, pt, px, r2, t, ix),
            -1 => {
            },
            _ => panic!(),
        }

        for &dx in &[-1, 0, 1] {
            let ix = ix + dx;
            let fx = fx + dx;

            // check cell centered in r1
            match sy {
                -1 => self.compute_checks2(&mut acc, r0, r1, Some(r2), t, ix, r0, ft, fx),
                0 => self.compute_checks2(&mut acc, r0, r1, Some(r2), t, ix, r1, ft, fx),
                1 => self.compute_checks2(&mut acc, r0, r1, Some(r2), t, ix, r2, ft, fx),
                _ => panic!(),
            }

            // check cell centered in n2b
            match sy {
                -1 => self.compute_checks2(&mut acc, r1, r2, None, t, ix, r1, ft, fx),
                0 => self.compute_checks2(&mut acc, r1, r2, None, t, ix, r2, ft, fx),
                1 => {
                },
                _ => panic!(),
            }
        }

        acc
    }

    pub fn derived<B: UScalar>(self) -> GolGraph<B> {
        let checks = (0..(self.mx * self.mt)).map(|idx| self.compute_checks(idx)).collect();

        GolGraph {
            mt: self.mt,
            mx: self.mx,
            wx: self.wx,

            left_sym: self.left_sym,
            right_sym: self.right_sym,

            checks: checks,

            recenter: self.recenter,
        }
    }
}

pub struct GolGraph<B: UScalar> {
    pub mt: usize,
    pub mx: usize,
    pub wx: usize,

    pub left_sym: GolSym,
    pub right_sym: GolSym,

    pub checks: Vec<Vec<(Vec<(usize, B)>, u32, (usize, B), (usize, B))>>,

    pub recenter: GolRecenter,
}

impl<B: UScalar> GolGraph<B> {
    fn to_idx(&self, x: usize, t: usize) -> usize {
        debug_assert!(x < self.mx);
        debug_assert!(t < self.mt);
        t * self.mx + x
    }

    fn collect_row(&self, pr: &mut PrintBag, row: B, x0: isize, y0: usize) {
        for t in 0..self.mt {
            for x in 0..self.mx {
                pr.insert(x0 + (x as isize), y0, t, match B::get_bit(&row, self.to_idx(x, t)) {
                    true => '*',
                    false => '.',
                });
            }
        }
    }

    fn collect_dash_row(&self, pr: &mut PrintBag, x0: isize, y0: usize) {
        for t in 0..self.mt {
            for x in 0..self.mx {
                pr.insert(x0 + (x as isize), y0, t, '-');
            }
        }
    }

    pub fn format_rows(&self, rows: &Vec<GolKeyNode<B>>) -> Vec<String> {
        let mut pr = PrintBag::new(self.mt);
        let mut y = 0;
        for (n, row) in rows.iter().enumerate() {
            if n == rows.len() - 1 {
                // last, output both
                self.collect_row(&mut pr, row.r0, row.dx, y);
                self.collect_row(&mut pr, row.r1, row.dx, y + 1);
            }
            else {
                // output each first row before that exactly once
                self.collect_row(&mut pr, row.r0, row.dx, y);
                y += 1;
            }
        }
        pr.format()
    }

    pub fn format_cycle_rows(&self, path: &Vec<GolKeyNode<B>>, cycle: &Vec<GolKeyNode<B>>, last: &GolKeyNode<B>) -> Vec<String> {
        // Just need to output each first row once (since cycle continues forever).
        let mut pr = PrintBag::new(self.mt);
        let mut y = 0;
        for row in path.iter() {
            self.collect_row(&mut pr, row.r0, row.dx, y);
            y += 1;
        }
        for (n, row) in cycle.iter().enumerate() {
            if n == 0 {
                self.collect_dash_row(&mut pr, row.dx, y);
                y += 1;
            }
            self.collect_row(&mut pr, row.r0, row.dx, y);
            y += 1;
        }
        self.collect_dash_row(&mut pr, last.dx, y);
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

fn find_min_x<B: UScalar>(e: &GolGraph<B>, r: B) -> usize {
    for x in 0..e.mx {
        for t in 0..e.mt {
            if r.get_bit(e.to_idx(x, t)) {
                return x;
            }
        }
    }

    e.mx - 1
}

fn find_max_x<B: UScalar>(e: &GolGraph<B>, r: B) -> usize {
    for x in (0..e.mx).rev() {
        for t in 0..e.mt {
            if r.get_bit(e.to_idx(x, t)) {
                return x;
            }
        }
    }

    0
}

pub fn recenter<B: UScalar>(e: &GolGraph<B>, r0: B, r1: B) -> (isize, B, B) {
    let bias = match e.recenter {
        GolRecenter::None => {
            return (0, r0, r1);
        }
        GolRecenter::BiasLeft => 0,
        GolRecenter::BiasRight => 1,
    };

    let r = (r0 | r1);
    if r == B::zero() {
        return (0, r0, r1);
    }

    let min_x = find_min_x(e, r) as isize;
    let max_x = find_max_x(e, r) as isize;

    let shift = ((min_x + max_x) - (0 + (e.mx as isize) - 1) + bias).div_euclid(2);

    let mut r0s = B::zero();
    let mut r1s = B::zero();
    for x in 0..e.mx {
        let ix = x as isize;
        for t in 0..e.mt {
            if r0.get_bit(e.to_idx(x, t)) {
                r0s.set_bit(e.to_idx((ix - shift) as usize, t), true);
            }
            if r1.get_bit(e.to_idx(x, t)) {
                r1s.set_bit(e.to_idx((ix - shift) as usize, t), true);
            }
        }
    }

    (shift, r0s, r1s)
}

fn expand_srch<B: UScalar>(e: &GolGraph<B>, n1: &GolNode<B>, n2s: &mut Vec<GolNode<B>>) {
    let idx = n1.r2l;

    if idx == e.mt * e.mx {
        let (shift, r0, r1) = recenter(e, n1.r1, n1.r2);

        if n1.r0 == B::zero() && n1.r1 == B::zero() && shift != 0 {
            // refuse since we'll find it anyway when we generate it already centered
            return;
        }

        n2s.push(GolNode {
            dx: n1.dx + shift,
            r0: r0,
            r1: r1,
            r2: B::zero(),
            r2_min_x: e.mx - 1,
            r2_max_x: 0,
            r2l: 0,
            r2l_x: 0,
        });
        return;
    }

    let x = n1.r2l_x;

    let mut n2 = GolNode {
        dx: n1.dx,
        r0: n1.r0,
        r1: n1.r1,
        r2: n1.r2,
        r2_min_x: n1.r2_min_x,
        r2_max_x: n1.r2_max_x,
        r2l: n1.r2l + 1,
        r2l_x: if n1.r2l_x == e.mx - 1 { 0 } else { n1.r2l_x + 1},
    };
    'v: for &v in &[false, true] {
        if v {
            let r2_min_x = n1.r2_min_x.min(x);
            let r2_max_x = n1.r2_max_x.max(x);
            if r2_max_x >= r2_min_x + e.wx {
                continue;
            }
            n2.r2_min_x = r2_min_x;
            n2.r2_max_x = r2_max_x;
        }
        else {
            n2.r2_min_x = n1.r2_min_x;
            n2.r2_max_x = n1.r2_max_x;
        }
        n2.r2.set_bit(idx, v);

        let rows = [n2.r0, n2.r1, n2.r2];

        for &(ref nh_masks, nh_ct, (cur_row_idx, cur_mask), (fut_row_idx, fut_mask)) in e.checks[idx].iter() {
            let mut nh = 0;
            for &(nh_row_idx, nh_mask) in nh_masks {
                nh += (rows[nh_row_idx] & nh_mask).count_ones()
            }

            let cur_cell = (rows[cur_row_idx] & cur_mask != B::zero());
            let fut_cell = (rows[fut_row_idx] & fut_mask != B::zero());

            if !check_compat2(nh, nh_ct, cur_cell, fut_cell) {
                continue 'v;
            }
        }

        n2s.push(n2.clone());
    }
}

impl<B: UScalar> DfsGraph<GolNode<B>> for GolGraph<B> {
    fn expand(&self, n1: &GolNode<B>) -> Vec<GolNode<B>> {
        let mut n2s = Vec::new();
        expand_srch(self, n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &GolKeyNode<B>) -> bool {
        (n.r0 == B::zero()) && (n.r1 == B::zero())
    }
}
