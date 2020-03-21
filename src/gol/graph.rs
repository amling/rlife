use crate::bits;
use crate::dfs;

use bits::Bits;
use dfs::graph::DfsGraph;

pub struct GolGraph {
    pub mt: usize,
    pub mx: usize,

    pub ox: isize,
    pub oy: isize,
}

impl GolGraph {
    pub fn print_row<B: Bits>(&self, row: B) {
        let mut r = String::new();
        for t in 0..self.mt {
            if t != 0 {
                r.push(' ');
            }

            for x in 0..self.mx {
                r.push(match B::get_bit(&row, x * self.mt + t) {
                    true => '*',
                    false => '.',
                });
            }
        }
        println!("{}", r);
    }

    pub fn print_prow<B: Bits>(&self, row: PartialRow<B>) {
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
        println!("{}", r);
    }

    pub fn print_rows<B: Bits>(&self, rows: &Vec<(B, B, B, usize)>) {
        for row in rows {
            self.print_row(row.1);
        }
    }

    pub fn print_dash_row(&self) {
        let mut r = String::new();
        for t in 0..self.mt {
            if t != 0 {
                r.push(' ');
            }

            for _x in 0..self.mx {
                r.push('-');
            }
        }
        println!("{}", r);
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

        if x < 0 || x >= (e.mx as isize) {
            return Some(false);
        }

        let idx = (x as usize) * e.mt + t;
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
            // need 2 or 3
            return cts.living <= 3 && cts.dead <= 6;
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

fn expand_srch<B: Bits>(e: &GolGraph, n1: &(B, B, B, usize), n2s: &mut Vec<(B, B, B, usize)>) {
    let idx = n1.3;
    let x = idx / e.mt;
    let t = idx % e.mt;

    let mut n2 = n1.clone();
    for &v in &[true, false] {
        Bits::set_bit(&mut n2.2, idx, v);

        let r0 = PartialRow::full(e, n2.0);
        let r1 = PartialRow::full(e, n2.1);
        let r2 = PartialRow::new(n2.2, idx + 1);
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
            n2s.push(n2);
        }
    }
}

impl<B: Bits> DfsGraph<(B, B, B, usize)> for GolGraph {
    fn start(&self) -> (B, B, B, usize) {
        assert!(self.mt * self.mx <= B::size());
        (B::zero(), B::zero(), B::zero(), 0)
    }

    fn expand(&self, n1: &(B, B, B, usize)) -> Vec<(B, B, B, usize)> {
        let mut n2s = Vec::new();
        expand_srch(self, n1, &mut n2s);
        n2s
    }

    fn end(&self, n: &(B, B, B, usize)) -> bool {
        return (n.3 == self.mt * self.mx) && n.1 == B::zero() && n.2 == B::zero();
    }
}
