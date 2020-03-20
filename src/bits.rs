trait Bits {
    fn zero() -> Self;
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

uxx_bits_impl!(u32, 32);
uxx_bits_impl!(u64, 64);
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
