use std::marker::PhantomData;

use crate::bits;
use crate::dfs;

use bits::Bits;
use dfs::graph::DfsGraphConfig;

struct GolGraphEnv {
    mt: usize,
    mx: usize,

    ox: isize,
    oy: isize,
}

struct GolGraphConfig<B> {
    _b: PhantomData<B>,
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

    fn full(e: &GolGraphEnv, bits: B) -> Self {
        Self::new(bits, e.mx * e.mt)
    }

    fn empty() -> Self {
        Self::new(B::zero(), 0)
    }
}

fn compute_shift(t: usize, mt: usize, o: isize) -> isize {
    // cumulative shift after t steps can be floor(o * t / mt) so we diff cumulatives
    let t = t as isize;
    let mt = mt as isize;
    let before = (o * t) / mt;
    let after = (o * (t + 1)) / mt;
    return after - before;
}

fn check_compat<B: Bits>(e: &GolGraphEnv, cp: PartialRow<B>, c: PartialRow<B>, cn: PartialRow<B>, ct: usize, cx: isize, f: PartialRow<B>, ft: usize, fx: isize) -> bool {
    unimplemented!();
}

fn expand_srch<B: Bits>(e: &GolGraphEnv, n1: &(B, B), n2s: &mut Vec<(B, B)>, n2b: &mut B, mut x: usize, mut t: usize) {
    if t == e.mt {
        t = 0;
        x += 1;

        if x == e.mx {
            n2s.push((n1.1, *n2b));
            return;
        }
    }

    for &v in &[true, false] {
        let idx = x * e.mt + t;
        Bits::set_bit(n2b, idx, v);

        let n10r = PartialRow::full(e, n1.0);
        let n11r = PartialRow::full(e, n1.1);
        let n2br = PartialRow::new(*n2b, idx);
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
            1 => check_compat(e, n10r, n11r, n2br, pt, px, n2br, t, ix),
            0 => check_compat(e, n11r, n2br, er, pt, px, n2br, t, ix),
            -1 => true,
            _ => panic!(),
        };

        for &dx in &[-1, 0, 1] {
            let ix = ix + dx;
            let fx = fx + dx;

            // check cell centered in n1.1
            ok &= match sy {
                -1 => check_compat(e, n10r, n11r, n2br, t, ix, n10r, ft, fx),
                0 => check_compat(e, n10r, n11r, n2br, t, ix, n11r, ft, fx),
                1 => check_compat(e, n10r, n11r, n2br, t, ix, n2br, ft, fx),
                _ => panic!(),
            };

            // check cell centered in n2b
            ok &= match sy {
                -1 => check_compat(e, n11r, n2br, er, t, ix, n11r, ft, fx),
                0 => check_compat(e, n11r, n2br, er, t, ix, n2br, ft, fx),
                1 => true,
                _ => panic!(),
            };
        }

        if !ok {
            continue;
        }

        expand_srch(e, n1, n2s, n2b, x, t + 1);
    }
}

impl<B: Bits> DfsGraphConfig for GolGraphConfig<B> {
    type E = GolGraphEnv;
    type N = (B, B);

    fn start(e: &GolGraphEnv) -> (B, B) {
        assert!(e.mt * e.mx <= B::size());
        <(B, B)>::zero()
    }

    fn expand(e: &GolGraphEnv, n1: &(B, B)) -> Vec<(B, B)> {
        let mut n2b = B::zero();
        let mut n2s = Vec::new();
        expand_srch(e, n1, &mut n2s, &mut n2b, 0, 0);
        n2s
    }

    fn end(_e: &GolGraphEnv, n: &(B, B)) -> bool {
        return *n == <(B, B)>::zero();
    }
}
