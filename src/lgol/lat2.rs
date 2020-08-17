use ars_aa::lattice::LatticeCanonicalizable;
use ars_aa::lattice::LatticeCanonicalizer;
use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::lgol;

use lgol::bg::LGolBgCoord;
use lgol::lat1::LGolLat1;
use lgol::lat1::Vec3;

pub struct LGolShiftData<BC> {
    pub adet: isize,
    pub w_bg_coord: BC,
    pub period: isize,
    pub bg_period: BC,
    pub shift_rows: Vec<Vec<(usize, BC)>>,
    pub checks: Vec<(isize, usize, BC)>,
    pub min_coord: isize,
    pub max_coord: isize,
    pub division_walks: Vec<Option<Vec<usize>>>,
}

pub struct LGolLat2<BC: LGolBgCoord> {
    pub spots: Vec<(Vec3, Vec3, BC)>,
    pub w_bg_coord: BC,
    pub u_shift_data: LGolShiftData<BC>,
    pub v_shift_data: LGolShiftData<BC>,
}

impl<BC: LGolBgCoord> LGolLat2<BC> {
    pub fn new(lat1: &LGolLat1) -> LGolLat2<BC> {
        // step two: figure out (x, y, t) coordinates for fundamental volume
        let mut spots = Vec::new();
        for t in 0..lat1.mt {
            for x in 0..lat1.mx {
                for y in 0..lat1.my {
                    // this (x, y, t) is some equivalence class, but we want to shift it to be in
                    // [0, 1)x[0, 1)x[0, 1) in uvw space

                    let (u, v, w) = lat1.xyt_to_uvw((x, y, t));

                    // adjust into [0, 1)x[0, 1)x[0, 1)
                    let u = u.rem_euclid(lat1.adet);
                    let v = v.rem_euclid(lat1.adet);
                    let w = w.rem_euclid(lat1.adet);
                    let uvw = (u, v, w);
                    let xyt = lat1.uvw_to_xyt(uvw);

                    spots.push((xyt, uvw, BC::from_xyt(xyt)));
                }
            }
        }

        spots.sort_by_key(|&(_, (u, v, _w), _)| {
            // Ugh, in order to get u and v handled sanely in the "single w layer" case we have to
            // ignore w (since some of them are shifted up in w space).  This will likely need
            // revisitting when/if we do any "multiple w layer" searches...
            (u, v)
        });

        // immutable
        let spots = spots;

        let compute_shift_data = |v_us: Vec3, mangle: &dyn Fn(Vec3) -> Vec3| {
            let period = {
                let v1 = mangle(lat1.xyt_to_uvw((1, 0, 0)));
                let v2 = mangle(lat1.xyt_to_uvw((0, 1, 0)));
                let v3 = mangle(lat1.xyt_to_uvw((0, 0, 1)));

                let l3 = Vec3::canonicalize(vec![v1, v2, v3]);
                let (_, (_, (lc, ()))) = l3;
                lc.unwrap().0
            };

            let mut rows = BTreeMap::new();
            for (idx, &(_xyt, uvw, bg_coord)) in spots.iter().enumerate() {
                let (c, other, w) = mangle(uvw);
                rows.entry((other, w)).or_insert_with(|| BTreeMap::new()).insert(c, (idx, bg_coord));
            }

            let shift_rows: Vec<Vec<_>> = rows.into_iter().map(|(_row_key, row)| {
                let row: Vec<_> = row.into_iter().collect();
                for i in 0..(row.len() - 1) {
                    assert_eq!(row[i].0 + period, row[i + 1].0);
                }
                row.into_iter().map(|(_c, (idx, bg_coord))| {
                    (idx, bg_coord)
                }).collect()
            }).collect();

            let mut checks: Vec<_> = spots.iter().enumerate().map(|(idx, &(_xyt, uvw, bg_coord))| {
                let c = mangle(uvw).0;
                (c, idx, bg_coord)
            }).collect();
            checks.sort();
            let min_coord = spots.iter().map(|&(_xyt, uvw, _bg_coord)| mangle(uvw).0).min().unwrap();
            let max_coord = spots.iter().map(|&(_xyt, uvw, _bg_coord)| mangle(uvw).0).max().unwrap();

            let bg_period = {
                let uvw = mangle((period, 0, 0));
                let xyt = lat1.uvw_to_xyt(uvw);
                BC::from_xyt(xyt)
            };

            let (x_us, y_us, t_us) = v_us;
            let max_divison_walk = x_us.abs().max(y_us.abs()).max(t_us.abs());
            let division_walks = (0..=max_divison_walk).map(|division| {
                if division < 2 {
                    return None;
                }
                if x_us.rem_euclid(division) != 0 {
                    return None
                }
                if y_us.rem_euclid(division) != 0 {
                    return None
                }
                if t_us.rem_euclid(division) != 0 {
                    return None
                }

                let step = (x_us / division, y_us / division, t_us / division);
                let l_step = Vec3::canonicalize(vec![step]);
                let mut buckets = HashMap::new();
                for (idx, &(xyt, _, _)) in spots.iter().enumerate() {
                    let xytc = l_step.canonicalize(xyt);
                    buckets.entry(xytc).or_insert_with(|| Vec::new()).push(idx);
                }
                let mut prev_map = HashMap::new();
                for (_, mut idxs) in buckets.into_iter() {
                    idxs.sort();
                    for (i, &idx) in idxs.iter().enumerate() {
                        let idx_prev = match i {
                            0 => idxs[idxs.len() - 1],
                            _ => idxs[i - 1],
                        };
                        assert!(prev_map.insert(idx, idx_prev).is_none());
                    }
                }

                let prevs = (0..spots.len()).map(|idx| prev_map[&idx]).collect();

                Some(prevs)
            }).collect();

            LGolShiftData {
                adet: lat1.adet,
                w_bg_coord: BC::from_xyt(lat1.w_to_xyt),
                period: period,
                bg_period: bg_period,
                shift_rows: shift_rows,
                checks: checks,
                min_coord: min_coord,
                max_coord: max_coord,
                division_walks: division_walks,
            }
        };

        let u_shift_data = compute_shift_data(lat1.u_to_xyt, &|uvw| uvw);
        let v_shift_data = compute_shift_data(lat1.v_to_xyt, &|(u, v, w)| (v, u, w));

        LGolLat2 {
            spots: spots,
            w_bg_coord: BC::from_xyt(lat1.w_to_xyt),
            u_shift_data: u_shift_data,
            v_shift_data: v_shift_data,
        }
    }
}
