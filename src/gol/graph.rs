use crate::bits;
use crate::dfs;

use bits::Bits;
use dfs::graph::DfsGraph;

#[derive(Clone)]
#[derive(Eq)]
#[derive(Hash)]
#[derive(PartialEq)]
pub struct GolNode<B> {
    r0: B,
    r1: B,
    r2: B,
    r2l: usize,
}

pub struct GolGraph {
    pub mt: usize,
    pub mx: usize,

    pub ox: isize,
    pub oy: isize,
}

impl GolGraph {
    fn to_idx(&self, x: usize, t: usize) -> usize {
        t * self.mx + x
    }

    fn x_from_idx(&self, idx: usize) -> usize {
        idx % self.mx
    }

    fn t_from_idx(&self, idx: usize) -> usize {
        idx / self.mx
    }

    pub fn format_row<B: Bits>(&self, row: B) -> String {
        let mut r = String::new();
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

    fn format_prow<B: Bits>(&self, row: PartialRow<B>) -> String {
        let mut r = String::new();
        for t in 0..self.mt {
            if t != 0 {
                r.push(' ');
            }

            for x in 0..self.mx {
                r.push(match row.get(self, t, x as isize) {
                    Some(true) => '*',
                    Some(false) => '.',
                    None => '?',
                });
            }
        }
        r
    }

    pub fn format_rows<B: Bits>(&self, rows: &Vec<GolNode<B>>) -> Vec<String> {
        let mut ret = Vec::new();
        for (n, row) in rows.iter().enumerate() {
            if n == rows.len() - 1 {
                // last, output everything even if partial
                ret.push(self.format_row(row.r0));
                ret.push(self.format_row(row.r1));
                ret.push(self.format_prow(PartialRow::new(row.r2, row.r2l)));
            }
            else if row.r2l == self.mt * self.mx {
                // output each first row before that exactly once (as third row fills)
                ret.push(self.format_row(row.r0));
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

    pub fn format_cycle_rows<B: Bits>(&self, path: &Vec<GolNode<B>>, cycle: &Vec<GolNode<B>>) -> Vec<String> {
        // Just need to output each first row once (since cycle continues forever).  Either third
        // row empty or full would do, but empty is easier.
        let mut ret = Vec::new();
        for row in path.iter() {
            if row.r2l == 0 {
                ret.push(self.format_row(row.r0));
            }
        }
        ret.push(self.format_dash_row());
        for row in cycle.iter() {
            if row.r2l == 0 {
                ret.push(self.format_row(row.r0));
            }
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
        if x < -1 {
            x = -2 - x;
        }
        if x == -1 {
            return Some(false);
        }
        let mx = e.mx as isize;
        if x >= mx {
            x = 2 * mx - 2 - x;
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

fn expand_srch<B: Bits>(e: &GolGraph, n1: &GolNode<B>, n2s: &mut Vec<GolNode<B>>) {
    let idx = n1.r2l;

    if idx == e.mt * e.mx {
        n2s.push(GolNode {
            r0: n1.r1,
            r1: n1.r2,
            r2: B::zero(),
            r2l: 0,
        });
        return;
    }

    let x = e.x_from_idx(idx);
    let t = e.t_from_idx(idx);

    let mut n2 = GolNode {
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

impl<B: Bits> DfsGraph<GolNode<B>> for GolGraph {
    fn start(&self) -> GolNode<B> {
        assert!(self.mt * self.mx <= B::size());
        GolNode {
            r0: B::c(0b0000010001000100110010000000001000010001010101010101110110010011001100100000),
            r1: B::c(0b0100010011001000000000100001000101010101010111011001001100110010000000000100),
            r2: B::zero(),
            r2l: 0,
        }
    }

    fn expand(&self, n1: &GolNode<B>) -> Vec<GolNode<B>> {
        let mut n2s = Vec::new();
        expand_srch(self, n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &GolNode<B>) -> bool {
        (n.r2l == self.mt * self.mx) && (n.r1 == B::zero()) && (n.r2 == B::zero())
    }
}
