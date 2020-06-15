use ars_ds::nice::Nice;
use ars_ds::scalar::Scalar;
use std::collections::HashMap;
use std::collections::HashSet;

use crate::lgol;

use lgol::bg::LGolBgCoord;
use lgol::graph::LGolHashNode;
use lgol::graph::RowTuple;

pub trait LGolEnds<BS: RowTuple, BC> {
    fn want_justify(&self) -> bool {
        false
    }

    fn end(&self, n: &LGolHashNode<BS, BC>) -> Option<&str>;
}

impl<BS: RowTuple, BC> LGolEnds<BS, BC> for () {
    fn end(&self, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        for &r in n.rs.as_slice() {
            if r != BS::Item::zero() {
                return None;
            }
        }
        Some("")
    }
}

impl<BS: RowTuple, BC: Nice> LGolEnds<BS, BC> for HashSet<LGolHashNode<BS, BC>> {
    fn end(&self, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        if self.contains(n) {
            Some("")
        }
        else {
            None
        }
    }
}

impl<BS: RowTuple, BC: Nice, S: AsRef<str>> LGolEnds<BS, BC> for HashMap<LGolHashNode<BS, BC>, S> {
    fn end(&self, n: &LGolHashNode<BS, BC>) -> Option<&str> {
        self.get(n).map(|s| s.as_ref())
    }
}

pub struct LGolNoEnds();

impl<BS: RowTuple, BC> LGolEnds<BS, BC> for LGolNoEnds {
    fn end(&self, _n: &LGolHashNode<BS, BC>) -> Option<&str> {
        None
    }
}

pub struct LGolMaskEnds<BS, BC, S>(HashMap<BC, HashMap<BS, HashMap<BS, S>>>);

impl<BS: RowTuple, BC: LGolBgCoord, S: AsRef<str>> LGolEnds<BS, BC> for LGolMaskEnds<BS, BC, S> {
    fn want_justify(&self) -> bool {
        true
    }

    fn end(&self, n: &LGolHashNode<BS, BC>) -> Option<&str> {
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
