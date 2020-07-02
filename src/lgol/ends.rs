use ars_ds::scalar::Scalar;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::lgol;

use lgol::bg::LGolBgCoord;
use lgol::graph::LGolHashNode;
use lgol::graph::RowTuple;
use lgol::lat2::LGolLat2;

pub trait LGolEnds<BS: RowTuple, BC: LGolBgCoord> {
    fn want_justify(&self) -> bool {
        false
    }

    fn end(&self, lat2: &LGolLat2<BC>, n: &LGolHashNode<BS, BC>) -> Option<&str>;
}

impl<BS: RowTuple, BC: LGolBgCoord> LGolEnds<BS, BC> for () {
    fn end(&self, _lat2: &LGolLat2<BC>, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        for &r in n.rs.as_slice() {
            if r != BS::Item::zero() {
                return None;
            }
        }
        Some("")
    }
}

impl<BS: RowTuple, BC: LGolBgCoord> LGolEnds<BS, BC> for HashSet<LGolHashNode<BS, BC>> {
    fn end(&self, _lat2: &LGolLat2<BC>, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        if self.contains(n) {
            Some("")
        }
        else {
            None
        }
    }
}

impl<BS: RowTuple, BC: LGolBgCoord, S: AsRef<str>> LGolEnds<BS, BC> for HashMap<LGolHashNode<BS, BC>, S> {
    fn end(&self, _lat2: &LGolLat2<BC>, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        self.get(n).map(|s| s.as_ref())
    }
}

pub struct LGolNoEnds();

impl<BS: RowTuple, BC: LGolBgCoord> LGolEnds<BS, BC> for LGolNoEnds {
    fn end(&self, _lat2: &LGolLat2<BC>, _n: &LGolHashNode<BS, BC>) -> Option<&str> {
        None
    }
}

pub struct LGolMaskEnds<BS, BC, S>(HashMap<BC, HashMap<BS, HashMap<BS, S>>>);

impl<BS: RowTuple, BC: LGolBgCoord, S: AsRef<str>> LGolEnds<BS, BC> for LGolMaskEnds<BS, BC, S> {
    fn want_justify(&self) -> bool {
        true
    }

    fn end(&self, _lat2: &LGolLat2<BC>, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        let bg_coord = n.bg_coord;
        let rs = n.rs;
        if let Some(m) = self.0.get(&bg_coord) {
            for (mask, m) in m.iter() {
                let mut rs = rs;
                rs.mask(mask);
                if let Some(s) = m.get(&rs) {
                    return Some(s.as_ref());
                }
            }
        }
        None
    }
}

impl<BS: RowTuple, BC: LGolBgCoord, S> LGolMaskEnds<BS, BC, S> {
    #[allow(dead_code)]
    pub fn new() -> LGolMaskEnds<BS, BC, S> {
        LGolMaskEnds(HashMap::new())
    }

    #[allow(dead_code)]
    pub fn add(&mut self, bg_coord: BC, mask: BS, pat: BS, s: S) {
        {
            let mut masked = pat;
            masked.mask(&mask);
            assert_eq!(pat, masked);
        }

        let m = &mut self.0;
        let m = m.entry(bg_coord).or_insert_with(|| HashMap::new());
        let m = m.entry(mask).or_insert_with(|| HashMap::new());
        m.insert(pat, s);
    }
}

pub struct LGolVPeriodDividingEnds(pub usize);

impl<BS: RowTuple, BC: LGolBgCoord> LGolEnds<BS, BC> for LGolVPeriodDividingEnds {
    fn end(&self, lat2: &LGolLat2<BC>, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        let division_walks = lat2.v_shift_data.division_walks[self.0].as_ref().unwrap();
        for (idx, &prev_idx) in division_walks.iter().enumerate() {
            for r in n.rs.as_slice() {
                if r.get_bit(idx) != r.get_bit(prev_idx) {
                    return None
                }
            }
        }
        Some("")
    }
}
