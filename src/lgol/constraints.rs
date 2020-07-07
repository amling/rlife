#![allow(unused_parens)]

use ars_ds::nice::Nice;

use crate::lgol;

use lgol::axis::LGolAxis;
use lgol::bg::LGolBgCoord;
use lgol::ends::LGolEnds;
use lgol::graph::LGolGraph;
use lgol::graph::RowTuple;

pub trait LGolConstraint<BC: LGolBgCoord>: Copy {
    type S: Nice;

    fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Self::S;
    fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, s0: Self::S, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Self::S>;
}

macro_rules! impl_cons {
    ($([$cs_id:ident / $s_id:ident / $ty:ident])*) => {
        impl<BC: LGolBgCoord $(, $ty: LGolConstraint<BC>)*> LGolConstraint<BC> for ($($ty,)*) {
            type S = ($($ty::S,)*);

            #[allow(unused_variables)]
            fn zero_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>) -> Self::S {
                let ($(ref $cs_id,)*) = self;
                ($($cs_id.zero_stat(ge),)*)
            }

            #[allow(unused_variables)]
            fn add_stat<BS: RowTuple>(&self, ge: &LGolGraph<BS, BC, impl LGolAxis<BC>, impl LGolAxis<BC>, impl LGolConstraint<BC>, impl LGolEnds<BS, BC>>, ($($s_id,)*): Self::S, bg_coord: BC, r: BS::Item, idx: usize, v: bool) -> Option<Self::S> {
                let ($(ref $cs_id,)*) = self;
                Some((
                    $(
                        match $cs_id.add_stat(ge, $s_id, bg_coord, r, idx, v) {
                            Some(s) => s,
                            None => {
                                return None;
                            },
                        },
                    )*
                ))
            }
        }
    }
}

impl_cons!();
impl_cons!([cs1 / s1 / CS1]);
impl_cons!([cs1 / s1 / CS1][cs2 / s2 / CS2]);
impl_cons!([cs1 / s1 / CS1][cs2 / s2 / CS2][cs3 / s3 / CS3]);
