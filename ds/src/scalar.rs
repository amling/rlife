use std::hash::Hash;
use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOrAssign;
use std::ops::Not;
use std::ops::Shl;
use std::ops::ShlAssign;
use std::ops::Shr;
use std::ops::ShrAssign;
use std::ops::Sub;

marker_trait! {
    ScalarOps:
    [BitAnd<Output=Self>]
    [BitAndAssign]
    [BitOrAssign]
    [Not<Output=Self>]
    [Shl<usize, Output=Self>]
    [ShlAssign<usize>]
    [Shr<usize, Output=Self>]
    [ShrAssign<usize>]
    [Sized]
    [Sub<Output=Self>]
}

pub trait Scalar: ScalarOps + Copy + Eq + Hash + Send + Sync {
    fn size() -> usize;
    fn zero() -> Self;
    fn one() -> Self;
    fn from_usize(c: usize) -> Self;
    fn to_usize(self) -> usize;
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
