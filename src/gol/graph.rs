use ars_ds::bit_state::Bits;
use serde::Deserialize;
use serde::Serialize;

use crate::dfs;

use dfs::graph::DfsGraph;

#[derive(Clone)]
#[derive(Deserialize)]
#[derive(Serialize)]
pub struct GolNode<B> {
    pub dx: isize,
    pub r0: B,
    pub r1: B,
    pub r2: B,
    pub r2l: usize,
}

#[derive(Clone)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct GolKeyNode<B> {
    pub dx: isize,
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
pub struct GolGraph {
    pub mt: usize,
    pub mx: usize,

    pub left_sym: GolSym,
    pub right_sym: GolSym,

    pub ox: isize,
    pub oy: isize,

    pub recenter: GolRecenter,
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

    pub fn format_row<B: Bits>(&self, dx: isize, row: B) -> String {
        let mut r = String::new();
        r.push_str(&format!("[{}] ", dx));
        for t in 0..self.mt {
            if t != 0 {
                r.push(' ');
            }

            for x in 0..self.mx {
                r.push(match B::get_bit(&row, self.to_idx(x, t)) {
                    true => '*',
                    false => '.',
                });
            }
        }
        r
    }

    pub fn format_rows<B: Bits>(&self, rows: &Vec<GolKeyNode<B>>) -> Vec<String> {
        let mut ret = Vec::new();
        for (n, row) in rows.iter().enumerate() {
            if n == rows.len() - 1 {
                // last, output both
                ret.push(self.format_row(row.dx, row.r0));
                ret.push(self.format_row(row.dx, row.r1));
            }
            else {
                // output each first row before that exactly once
                ret.push(self.format_row(row.dx, row.r0));
            }
        }
        ret
    }

    fn format_dash_row(&self) -> String {
        let mut r = String::new();
        for t in 0..self.mt {
            if t != 0 {
                r.push(' ');
            }

            for _x in 0..self.mx {
                r.push('-');
            }
        }
        r
    }

    pub fn format_cycle_rows<B: Bits>(&self, path: &Vec<GolKeyNode<B>>, cycle: &Vec<GolKeyNode<B>>) -> Vec<String> {
        // Just need to output each first row once (since cycle continues forever).
        let mut ret = Vec::new();
        for row in path.iter() {
            ret.push(self.format_row(row.dx, row.r0));
        }
        ret.push(self.format_dash_row());
        for row in cycle.iter() {
            ret.push(self.format_row(row.dx, row.r0));
        }
        ret
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
        assert!(t < e.mt);

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
}

fn compute_shift(t: usize, mt: usize, o: isize) -> isize {
    // cumulative shift after t steps can be floor(o * t / mt) so we diff cumulatives
    let t = t as isize;
    let mt = mt as isize;
    let before = (o * t) / mt;
    let after = (o * (t + 1)) / mt;
    after - before
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

fn check_compat<B: Bits>(e: &GolGraph, cp: PartialRow<B>, c: PartialRow<B>, cn: PartialRow<B>, ct: usize, cx: isize, f: PartialRow<B>, ft: usize, fx: isize) -> bool {
    let r = check_compat1(e, cp, c, cn, ct, cx, f, ft, fx);
    //eprintln!("check_compat(cp {}, c {}, cn {}, ct {}, cx {}, f {}, ft {}, fx {}) = {}", e.format_prow(cp), e.format_prow(c), e.format_prow(cn), ct, cx, e.format_prow(f), ft, fx, r);
    r
}

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

    0
}

fn find_max_x<B: Bits>(e: &GolGraph, r: B) -> usize {
    for x in (0..e.mx).rev() {
        for t in 0..e.mt {
            if r.get_bit(e.to_idx(x, t)) {
                return x;
            }
        }
    }

    e.mx - 1
}

fn recenter<B: Bits>(e: &GolGraph, dx: isize, r0: B, r1: B) -> (isize, B, B) {
    let bias = match e.recenter {
        GolRecenter::None => {
            return (dx, r0, r1);
        }
        GolRecenter::BiasLeft => 0,
        GolRecenter::BiasRight => 1,
    };

    let min_x = find_min_x(e, r0).min(find_min_x(e, r1)) as isize;
    let max_x = find_max_x(e, r0).max(find_max_x(e, r1)) as isize;

    let shift = ((min_x + max_x) - (0 + (e.mx as isize) - 1) + bias) / 2;

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

    (dx + shift, r0s, r1s)
}

fn expand_srch<B: Bits>(e: &GolGraph, n1: &GolNode<B>, n2s: &mut Vec<GolNode<B>>) {
    let idx = n1.r2l;

    if idx == e.mt * e.mx {
        let (dx, r0, r1) = recenter(e, n1.dx, n1.r1, n1.r2);

        n2s.push(GolNode {
            dx: dx,
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
    for &v in &[false, true] {
        Bits::set_bit(&mut n2.r2, idx, v);

        let r0 = PartialRow::full(e, n2.r0);
        let r1 = PartialRow::full(e, n2.r1);
        let r2 = PartialRow::new(n2.r2, idx + 1);
        let er = PartialRow::empty();

        let ix = x as isize;

        // shift for the previous generation
        let pt = (t + e.mt - 1) % e.mt;
        let sxp = compute_shift(pt, e.mt, e.ox);
        let syp = compute_shift(pt, e.mt, e.oy);
        let px = ix - sxp;

        // shift from this time to the next
        let ft = (t + 1) % e.mt;
        let sx = compute_shift(t, e.mt, e.ox);
        let sy = compute_shift(t, e.mt, e.oy);
        let fx = ix + sx;

        let mut ok = true;

        // check past cell if there is one (y shifts backwards!)
        ok &= match syp {
            1 => check_compat(e, r0, r1, r2, pt, px, r2, t, ix),
            0 => check_compat(e, r1, r2, er, pt, px, r2, t, ix),
            -1 => true,
            _ => panic!(),
        };

        for &dx in &[-1, 0, 1] {
            let ix = ix + dx;
            let fx = fx + dx;

            // check cell centered in n1.1
            ok &= match sy {
                -1 => check_compat(e, r0, r1, r2, t, ix, r0, ft, fx),
                0 => check_compat(e, r0, r1, r2, t, ix, r1, ft, fx),
                1 => check_compat(e, r0, r1, r2, t, ix, r2, ft, fx),
                _ => panic!(),
            };

            // check cell centered in n2b
            ok &= match sy {
                -1 => check_compat(e, r1, r2, er, t, ix, r1, ft, fx),
                0 => check_compat(e, r1, r2, er, t, ix, r2, ft, fx),
                1 => true,
                _ => panic!(),
            };
        }

        if ok {
            n2s.push(n2.clone());
        }
    }
}

impl<B: Bits> DfsGraph<GolNode<B>, GolKeyNode<B>> for GolGraph {
    fn expand(&self, n1: &GolNode<B>) -> Vec<GolNode<B>> {
        let mut n2s = Vec::new();
        expand_srch(self, n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &GolKeyNode<B>) -> bool {
        (n.r0 == B::zero()) && (n.r1 == B::zero())
    }

    fn key_for(&self, n: &GolNode<B>) -> Option<GolKeyNode<B>> {
        if n.r2l != 0 {
            return None;
        }

        Some(GolKeyNode {
            dx: n.dx,
            r0: n.r0,
            r1: n.r1,
        })
    }
}
