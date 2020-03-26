use std::fmt::Debug;
use std::hash::Hash;

pub trait Bits: Copy + Send + Sync + Hash + Eq + Debug {
    fn zero() -> Self;
    fn cnst(c: u128) -> Self {
        let mut b = Self::zero();
        let mut c = c;
        let mut idx = 0;
        while c > 0 {
            if c % 2 == 1 {
                assert!(idx < Self::size());
                Self::set_bit(&mut b, idx, true);
            }
            c >>= 1;
            idx += 1;
        }
        b
    }
    fn size() -> usize;
    fn get_bit(&self, n: usize) -> bool;
    fn set_bit(&mut self, n: usize, v: bool);
}

// I would love not to have to macro this mess, but rust's num traits suck so unbelievably badly.

macro_rules! uxx_bits_impl {
    ($t:ty, $n:expr) => {
        impl Bits for $t {
            fn zero() -> Self {
                0
            }

            fn size() -> usize {
                $n
            }

            fn get_bit(&self, n: usize) -> bool {
                ((*self >> n) & 1) != 0
            }

            fn set_bit(&mut self, n: usize, v: bool) {
                *self &= !(1 << n);
                if v {
                    *self |= 1 << n;
                }
            }
        }
    }
}

//uxx_bits_impl!(u32, 32);
//uxx_bits_impl!(u64, 64);
uxx_bits_impl!(u128, 128);

impl<A: Bits, B: Bits> Bits for (A, B) {
    fn zero() -> Self {
        (A::zero(), B::zero())
    }

    fn size() -> usize {
        A::size() + B::size()
    }

    fn get_bit(&self, n: usize) -> bool {
        if n < A::size() {
            self.0.get_bit(n)
        }
        else {
            self.1.get_bit(n - A::size())
        }
    }

    fn set_bit(&mut self, n: usize, v: bool) {
        if n < A::size() {
            self.0.set_bit(n, v)
        }
        else {
            self.1.set_bit(n - A::size(), v)
        }
    }
}
