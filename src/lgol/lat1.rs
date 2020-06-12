use ars_aa::lattice::LatticeCanonicalizable;

pub type Vec3 = (isize, isize, isize);

pub struct LGolLat1 {
    pub mx: isize,
    pub my: isize,
    pub mt: isize,

    u_to_xyt: Vec3,
    v_to_xyt: Vec3,
    w_to_xyt: Vec3,

    pub det: isize,
    pub adet: isize,
    pub sdet: isize,

    x_to_uvw: Vec3,
    y_to_uvw: Vec3,
    t_to_uvw: Vec3,
}

impl LGolLat1 {
    pub fn new(vu: Vec3, vv: Vec3, vw: Vec3) -> LGolLat1 {
        let l3 = Vec3::canonicalize(vec![vu, vv, vw]);
        let (la, (lb, (lc, ()))) = l3;
        let (_, _, mt) = la.unwrap();
        let (_, my) = lb.unwrap();
        let (mx,) = lc.unwrap();

        let (ux, uy, ut) = vu;
        let (vx, vy, vt) = vv;
        let (wx, wy, wt) = vw;

        let det = ux * (vy * wt - wy * vt) - uy * (vx * wt - wx * vt) + ut * (vx * wy - wx * vy);
        let sdet;
        if det < 0 {
            sdet = -1;
        }
        else if det > 0 {
            sdet = 1;
        }
        else {
            panic!();
        }
        assert_eq!(det * sdet, mx * my * mt);

        let (xu, xv, xw) = (vy * wt - wy * vt, wy * ut - uy * wt, uy * vt - vy * ut);
        let (yu, yv, yw) = (wx * vt - vx * wt, ux * wt - wx * ut, vx * ut - ux * vt);
        let (tu, tv, tw) = (vx * wy - wx * vy, wx * uy - ux * wy, ux * vy - vx * uy);

        debug_assert_eq!(xu * ux + xv * vx + xw * wx, det);
        debug_assert_eq!(xu * uy + xv * vy + xw * wy, 0);
        debug_assert_eq!(xu * ut + xv * vt + xw * wt, 0);
        debug_assert_eq!(yu * ux + yv * vx + yw * wx, 0);
        debug_assert_eq!(yu * uy + yv * vy + yw * wy, det);
        debug_assert_eq!(yu * ut + yv * vt + yw * wt, 0);
        debug_assert_eq!(tu * ux + tv * vx + tw * wx, 0);
        debug_assert_eq!(tu * uy + tv * vy + tw * wy, 0);
        debug_assert_eq!(tu * ut + tv * vt + tw * wt, det);

        LGolLat1 {
            mx: mx,
            my: my,
            mt: mt,

            u_to_xyt: (ux, uy, ut),
            v_to_xyt: (vx, vy, vt),
            w_to_xyt: (wx, wy, wt),

            det: det,
            adet: det * sdet,
            sdet: sdet,

            x_to_uvw: (xu, xv, xw),
            y_to_uvw: (yu, yv, yw),
            t_to_uvw: (tu, tv, tw),
        }
    }

    pub fn xyt_to_uvw(&self, (x, y, t): Vec3) -> Vec3 {
        let (xu, xv, xw) = self.x_to_uvw;
        let (yu, yv, yw) = self.y_to_uvw;
        let (tu, tv, tw) = self.t_to_uvw;

        // switch to uvw space (but with det denominator missing)
        let u = x * xu + y * yu + t * tu;
        let v = x * xv + y * yv + t * tv;
        let w = x * xw + y * yw + t * tw;

        // but return with sign flipped per det
        (u * self.sdet, v * self.sdet, w * self.sdet)
    }

    pub fn uvw_to_xyt(&self, (u, v, w): Vec3) -> Vec3 {
        let (ux, uy, ut) = self.u_to_xyt;
        let (vx, vy, vt) = self.v_to_xyt;
        let (wx, wy, wt) = self.w_to_xyt;

        let x = u * ux + v * vx + w * wx;
        let y = u * uy + v * vy + w * wy;
        let t = u * ut + v * vt + w * wt;

        assert_eq!(x % self.adet, 0);
        assert_eq!(y % self.adet, 0);
        assert_eq!(t % self.adet, 0);

        let x = x / self.adet;
        let y = y / self.adet;
        let t = t / self.adet;

        (x, y, t)
    }

    pub fn canonicalize_xyt(&self, (x, y, t): Vec3) -> (Vec3, Vec3) {
        let (mut u, mut v, mut w) = self.xyt_to_uvw((x, y, t));
        let mut lu = 0;
        let mut lv = 0;
        let mut lw = 0;

        // adjust into [0, 1)x[0, 1)x[0, 1)
        while u < 0 {
            u += self.adet;
            lu -= 1;
        }
        while u >= self.adet {
            u -= self.adet;
            lu += 1;
        }
        while v < 0 {
            v += self.adet;
            lv -= 1;
        }
        while v >= self.adet {
            v -= self.adet;
            lv += 1;
        }
        while w < 0 {
            w += self.adet;
            lw -= 1;
        }
        while w >= self.adet {
            w -= self.adet;
            lw += 1;
        }

        let (x, y, t) = self.uvw_to_xyt((u, v, w));

        ((x, y, t), (lu, lv, lw))
    }
}
