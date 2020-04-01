use ars_ds::tuple::CTupleEnd;

is_tuple_trait!(IsTuple);

pub trait ZModule {
    fn zero() -> Self;
    fn mul(&mut self, q: isize);
    fn addmul(&mut self, q: isize, b: &Self);
}

impl ZModule for () {
    fn zero() {
    }

    fn mul(&mut self, _q: isize) {
    }

    fn addmul(&mut self, _q: isize, _b: &Self) {
    }
}

impl ZModule for isize {
    fn zero() -> Self {
        0
    }

    fn mul(&mut self, q: isize) {
        *self *= q;
    }

    fn addmul(&mut self, q: isize, b: &Self) {
        *self += q * *b;
    }
}

impl<X: ZModule, Y: ZModule, T: CTupleEnd<F=X, B=Y> + Clone + IsTuple> ZModule for T {
    fn zero() -> Self {
        T::join_tuple_end(X::zero(), Y::zero())
    }

    fn mul(&mut self, q: isize) {
        let (mut x, mut y) = T::split_tuple_end(self.clone());
        x.mul(q);
        y.mul(q);
        *self = T::join_tuple_end(x, y);
    }

    fn addmul(&mut self, q: isize, b: &Self) {
        let (mut x, mut y) = T::split_tuple_end(self.clone());
        let (bx, by) = T::split_tuple_end(b.clone());
        x.addmul(q, &bx);
        y.addmul(q, &by);
        *self = T::join_tuple_end(x, y);
    }
}

