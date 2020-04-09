use ars_ds::bit_state::Bits;
use serde::Deserialize;
use serde::Serialize;

use crate::dfs;
use crate::gol;

use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use gol::printbag::PrintBag;

#[derive(Clone)]
#[derive(Debug)]
#[derive(Deserialize)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
#[derive(Serialize)]
pub struct GolNode<B: Bits> {
    pub dx: isize,
    pub r0: B,
    pub r1: B,
    pub r2: B,
    pub r2l: usize,
}

impl<B: Bits> DfsNode for GolNode<B> {
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
pub struct GolKeyNode<B: Bits> {
    pub dx: isize,
    pub r0: B,
    pub r1: B,
}

impl<B: Bits> DfsKeyNode for GolKeyNode<B> {
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
pub struct GolHashNode<B: Bits> {
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

    pub left_sym: GolSym,
    pub right_sym: GolSym,

    pub ox: isize,
    pub oy: isize,

    pub recenter: GolRecenter,
}

impl GolPreGraph {
    pub fn derived(self) -> GolGraph {
        let compute_shift = |t, o| {
            // cumulative shift after t steps can be floor(o * t / mt) so we diff cumulatives
            let t = t as isize;
            let mt = self.mt as isize;
            let before = (o * t) / mt;
            let after = (o * (t + 1)) / mt;
            after - before
        };

        let shifts = (0..self.mt).map(|t| {
            let sx = compute_shift(t, self.ox);
            let sy = compute_shift(t, self.oy);
            (sx, sy)
        }).collect();

        GolGraph {
            mt: self.mt,
            mx: self.mx,

            left_sym: self.left_sym,
            right_sym: self.right_sym,

            shifts: shifts,

            recenter: self.recenter,
        }
    }
}

pub struct GolGraph {
    mt: usize,
    mx: usize,

    left_sym: GolSym,
    right_sym: GolSym,

    shifts: Vec<(isize, isize)>,

    recenter: GolRecenter,
}

impl GolGraph {
    fn to_idx(&self, x: usize, t: usize) -> usize {
        debug_assert!(x < self.mx);
        debug_assert!(t < self.mt);
        t * self.mx + x
    }

    fn x_from_idx(&self, idx: usize) -> usize {
        debug_assert!(idx < self.mx * self.mt);
        idx % self.mx
    }

    fn t_from_idx(&self, idx: usize) -> usize {
        idx / self.mx
    }

    fn prev_t(&self, t: usize) -> usize {
        if t == 0 {
            return self.mt - 1;
        }
        t - 1
    }

    fn next_t(&self, t: usize) -> usize {
        if t == self.mt - 1 {
            return 0;
        }
        t + 1
    }

    fn collect_row<B: Bits>(&self, pr: &mut PrintBag, row: B, x0: isize, y0: usize) {
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

    pub fn format_rows<B: Bits>(&self, rows: &Vec<GolKeyNode<B>>) -> Vec<String> {
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

    pub fn format_cycle_rows<B: Bits>(&self, path: &Vec<GolKeyNode<B>>, cycle: &Vec<GolKeyNode<B>>, last: &GolKeyNode<B>) -> Vec<String> {
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

#[derive(Clone)]
#[derive(Copy)]
struct PartialRow<B: Bits> {
    bits: B,
    len: usize,
}

impl<B: Bits> PartialRow<B> {
    fn new(bits: B, len: usize) -> Self {
        PartialRow {
            bits: bits,
            len: len,
        }
    }

    fn full(e: &GolGraph, bits: B) -> Self {
        Self::new(bits, e.mx * e.mt)
    }

    fn empty() -> Self {
        Self::new(B::zero(), 0)
    }

    fn get(&self, e: &GolGraph, t: usize, x: isize) -> Option<bool> {
        debug_assert!(t < e.mt);

        let mut x = x;
        let mx = e.mx as isize;
        if x < 0 {
            x = match e.left_sym {
                GolSym::Empty => {
                    return Some(false);
                },
                GolSym::Odd => -x,
                GolSym::Even => -x - 1,
                GolSym::Gutter => {
                    if x == -1 {
                        return Some(false);
                    }
                    -x - 2
                }
                GolSym::Wrap => x + mx,
            };
        }
        if x >= mx {
            x = match e.right_sym {
                GolSym::Empty => {
                    return Some(false);
                },
                GolSym::Odd => 2 * mx - 2 - x,
                GolSym::Even => 2 * mx - 1 - x,
                GolSym::Gutter => {
                    if x == mx {
                        return Some(false);
                    }
                    2 * mx - x
                },
                GolSym::Wrap => x - mx,
            };
        }

        let idx = e.to_idx(x as usize, t);
        if idx >= self.len {
            return None;
        }

        return Some(Bits::get_bit(&self.bits, idx));
    }

    fn get_cts(&self, e: &GolGraph, t: usize, x: isize) -> CellCounts {
        match self.get(e, t, x) {
            Some(true) => CellCounts::new(1, 0),
            Some(false) => CellCounts::new(0, 1),
            None => CellCounts::new(0, 0),
        }
    }

    fn format(&self, e: &GolGraph) -> String {
        let mut s = String::new();
        for t in 0..e.mt {
            if t > 0 {
                s.push(' ');
            }
            for x in 0..e.mx {
                let idx = e.to_idx(x, t);
                let c = match idx < self.len {
                    true => match self.bits.get_bit(idx) {
                        true => '*',
                        false => '.',
                    },
                    false => '?',
                };
                s.push(c);
            }
        }
        s
    }
}

#[derive(Default)]
struct CellCounts {
    living: usize,
    dead: usize,
}

impl CellCounts {
    fn new(living: usize, dead: usize) -> Self {
        CellCounts {
            living: living,
            dead: dead,
        }
    }
}

impl std::ops::AddAssign for CellCounts {
    fn add_assign(&mut self, rhs: Self) {
        self.living += rhs.living;
        self.dead += rhs.dead;
    }
}

// I dislike trying to force the compiler's hand, but we really dearly value speed over the modest
// increase in binary size.
#[inline(always)]
fn check_compat<B: Bits>(e: &GolGraph, cp: PartialRow<B>, c: PartialRow<B>, cn: PartialRow<B>, ct: usize, cx: isize, f: PartialRow<B>, ft: usize, fx: isize) -> bool {
    let r = check_compat1(e, cp, c, cn, ct, cx, f, ft, fx);
//eprintln!("check_compat(cp {} c {} cn {} ct {} cx {} f {} ft {} fx {}) = {}", cp.format(e), c.format(e), cn.format(e), ct, cx, f.format(e), ft, fx, r);
    r
}

#[inline(always)]
fn check_compat1<B: Bits>(e: &GolGraph, cp: PartialRow<B>, c: PartialRow<B>, cn: PartialRow<B>, ct: usize, cx: isize, f: PartialRow<B>, ft: usize, fx: isize) -> bool {
    let mut cts = CellCounts::new(0, 0);

    cts += cp.get_cts(e, ct, cx - 1);
    cts += cp.get_cts(e, ct, cx);
    cts += cp.get_cts(e, ct, cx + 1);
    cts += c.get_cts(e, ct, cx - 1);
    cts += c.get_cts(e, ct, cx + 1);
    cts += cn.get_cts(e, ct, cx - 1);
    cts += cn.get_cts(e, ct, cx);
    cts += cn.get_cts(e, ct, cx + 1);

    let fs = match f.get(e, ft, fx) {
        Some(fs) => fs,
        None => {
            return true;
        }
    };

    let cs = match c.get(e, ct, cx) {
        Some(cs) => cs,
        None => {
            return match fs {
                // need 2 or 3
                true => cts.living <= 3 && cts.dead <= 6,

                // need not exactly 3
                false => cts.living != 3 || cts.dead != 5,
            };
        },
    };

    match cs {
        true => match fs {
            true => {
                // need 2 or 3
                cts.living <= 3 && cts.dead <= 6
            },
            false => {
                // need 0, 1, or 4+
                cts.living <= 1 || cts.dead <= 4
            },
        },
        false => match fs {
            true => {
                // need 3
                cts.living <= 3 && cts.dead <= 5
            },
            false => {
                cts.living <= 2 || cts.dead <= 4
            },
        },
    }
}

fn find_min_x<B: Bits>(e: &GolGraph, r: B) -> usize {
    for x in 0..e.mx {
        for t in 0..e.mt {
            if r.get_bit(e.to_idx(x, t)) {
                return x;
            }
        }
    }

    panic!();
}

fn find_max_x<B: Bits>(e: &GolGraph, r: B) -> usize {
    for x in (0..e.mx).rev() {
        for t in 0..e.mt {
            if r.get_bit(e.to_idx(x, t)) {
                return x;
            }
        }
    }

    panic!();
}

fn recenter<B: Bits>(e: &GolGraph, r0: B, r1: B) -> (isize, B, B) {
    let bias = match e.recenter {
        GolRecenter::None => {
            return (0, r0, r1);
        }
        GolRecenter::BiasLeft => 0,
        GolRecenter::BiasRight => 1,
    };

    let r = r0.or(&r1);
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

fn expand_srch<B: Bits>(e: &GolGraph, n1: &GolNode<B>, n2s: &mut Vec<GolNode<B>>) {
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
            r2l: 0,
        });
        return;
    }

    let x = e.x_from_idx(idx);
    let t = e.t_from_idx(idx);

    let mut n2 = GolNode {
        dx: n1.dx,
        r0: n1.r0,
        r1: n1.r1,
        r2: n1.r2,
        r2l: n1.r2l + 1,
    };
    'v: for &v in &[false, true] {
        Bits::set_bit(&mut n2.r2, idx, v);

        let r0 = PartialRow::full(e, n2.r0);
        let r1 = PartialRow::full(e, n2.r1);
        let r2 = PartialRow::new(n2.r2, idx + 1);
        let er = PartialRow::empty();

        let ix = x as isize;

        // shift for the previous generation
        let pt = e.prev_t(t);
        let (sxp, syp) = e.shifts[pt];
        let px = ix - sxp;

        // shift from this time to the next
        let ft = e.next_t(t);
        let (sx, sy) = e.shifts[t];
        let fx = ix + sx;

        // check past cell if there is one (y shifts backwards!)
        let b = match syp {
            1 => check_compat(e, r0, r1, r2, pt, px, r2, t, ix),
            0 => check_compat(e, r1, r2, er, pt, px, r2, t, ix),
            -1 => true,
            _ => panic!(),
        };
        if !b {
            continue;
        }

        for &dx in &[-1, 0, 1] {
            let ix = ix + dx;
            let fx = fx + dx;

            // check cell centered in n1.1
            let b = match sy {
                -1 => check_compat(e, r0, r1, r2, t, ix, r0, ft, fx),
                0 => check_compat(e, r0, r1, r2, t, ix, r1, ft, fx),
                1 => check_compat(e, r0, r1, r2, t, ix, r2, ft, fx),
                _ => panic!(),
            };
            if !b {
                continue 'v;
            }

            // check cell centered in n2b
            let b = match sy {
                -1 => check_compat(e, r1, r2, er, t, ix, r1, ft, fx),
                0 => check_compat(e, r1, r2, er, t, ix, r2, ft, fx),
                1 => true,
                _ => panic!(),
            };
            if !b {
                continue 'v;
            }
        }

        n2s.push(n2.clone());
    }
}

impl<B: Bits> DfsGraph<GolNode<B>> for GolGraph {
    fn expand(&self, n1: &GolNode<B>) -> Vec<GolNode<B>> {
        let mut n2s = Vec::new();
        expand_srch(self, n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &GolKeyNode<B>) -> bool {
        (n.r0 == B::zero()) && (n.r1 == B::zero())
    }
}
