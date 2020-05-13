#![allow(unused_parens)]

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOr;
use std::ops::BitOrAssign;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Not;
use std::ops::Shl;
use std::ops::ShlAssign;
use std::ops::Shr;
use std::ops::ShrAssign;
use std::ops::Sub;
use std::ops::SubAssign;

marker_trait! {
    ScalarMarker:
    [Add<Output=Self>]
    [AddAssign]
    [BitAnd<Output=Self>]
    [BitAndAssign]
    [BitOr<Output=Self>]
    [BitOrAssign]
    [Copy]
    [Debug]
    [Default]
    [Div<Output=Self>]
    [Eq]
    [Hash]
    [Mul<Output=Self>]
    [Not<Output=Self>]
    [Ord]
    [PartialOrd]
    [Send]
    [Shl<usize, Output=Self>]
    [ShlAssign<usize>]
    [Shr<usize, Output=Self>]
    [ShrAssign<usize>]
    [Sized]
    [Sub<Output=Self>]
    [SubAssign]
    [Sync]
}

pub trait Scalar: ScalarMarker {
    fn size() -> usize;
    fn zero() -> Self;
    fn one() -> Self;
    fn from_usize(c: usize) -> Self;
    fn to_usize(self) -> usize;
    fn count_ones(&self) -> u32;

    fn set_bit(&mut self, idx: usize, v: bool) {
        *self &= !(Self::one() << idx);
        if v {
            *self |= (Self::one() << idx);
        }
    }

    fn get_bit(&self, idx: usize) -> bool {
        (*self & (Self::one() << idx)) != Self::zero()
    }
}

// marker for unsigned
pub trait UScalar: Scalar {
}

macro_rules! uxx_scalar_impl {
    ($t:ty, $n:expr) => {
        impl Scalar for $t {
            fn size() -> usize {
                $n
            }

            fn zero() -> Self {
                0
            }

            fn one() -> Self {
                1
            }

            fn from_usize(c: usize) -> Self {
                c as Self
            }

            fn to_usize(self) -> usize {
                self as usize
            }

            fn count_ones(&self) -> u32 {
                (*self).count_ones()
            }
        }

        impl UScalar for $t {
        }
    }
}

uxx_scalar_impl!(u8, 8);
uxx_scalar_impl!(u16, 16);
uxx_scalar_impl!(u32, 32);
uxx_scalar_impl!(u64, 64);
uxx_scalar_impl!(u128, 128);

// wtf, I can't find a better way to get bit count...
uxx_scalar_impl!(usize, (std::usize::MAX.count_ones() as usize));
