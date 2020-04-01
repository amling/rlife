#[derive(Clone)]
#[derive(Eq)]
#[derive(PartialEq)]
pub struct Tuple1<T>(pub T);

pub trait CTupleEnd {
    type F;
    type B;

    fn split_tuple_end(zelf: Self) -> (Self::F, Self::B);
    fn join_tuple_end(f: Self::F, b: Self::B) -> Self;
}

impl<A> CTupleEnd for Tuple1<A> {
    type F = ();
    type B = A;

    fn split_tuple_end(Tuple1(a): Self) -> ((), A) {
        ((), a)
    }

    fn join_tuple_end(_: (), a: A) -> Self {
        Tuple1(a)
    }
}

impl<A, B> CTupleEnd for (A, B) {
    type F = Tuple1<A>;
    type B = B;

    fn split_tuple_end((a, b): (A, B)) -> (Tuple1<A>, B) {
        (Tuple1(a), b)
    }

    fn join_tuple_end(a: Tuple1<A>, b: B) -> (A, B) {
        (a.0, b)
    }
}

impl<A, B, C> CTupleEnd for (A, B, C) {
    type F = (A, B);
    type B = C;

    fn split_tuple_end((a, b, c): (A, B, C)) -> ((A, B), C) {
        ((a, b), c)
    }

    fn join_tuple_end((a, b): (A, B), c: C) -> (A, B, C) {
        (a, b, c)
    }
}

pub trait TupleEnd<B> {
    type F;

    fn split_tuple_end(zelf: Self) -> (Self::F, B);
    fn join_tuple_end(f: Self::F, b: B) -> Self;
}

impl<A> TupleEnd<A> for A {
    type F = ();

    fn split_tuple_end(a: Self) -> ((), A) {
        ((), a)
    }

    fn join_tuple_end(_: (), a: A) -> A {
        a
    }
}

impl<A, B> TupleEnd<B> for (A, B) {
    type F = A;

    fn split_tuple_end((a, b): (A, B)) -> (A, B) {
        (a, b)
    }

    fn join_tuple_end(a: A, b: B) -> (A, B) {
        (a, b)
    }
}

impl<A, B, C> TupleEnd<C> for (A, B, C) {
    type F = (A, B);

    fn split_tuple_end((a, b, c): (A, B, C)) -> ((A, B), C) {
        ((a, b), c)
    }

    fn join_tuple_end((a, b): (A, B), c: C) -> (A, B, C) {
        (a, b, c)
    }
}

pub trait CTupleStart {
    type F;
    type B;

    fn split_tuple_start(zelf: Self) -> (Self::F, Self::B);
    fn join_tuple_start(f: Self::F, b: Self::B) -> Self;
}

impl<A> CTupleStart for Tuple1<A> {
    type F = A;
    type B = ();

    fn split_tuple_start(Tuple1(a): Self) -> (A, ()) {
        (a, ())
    }

    fn join_tuple_start(a: A, _: ()) -> Self {
        Tuple1(a)
    }
}

impl<A, B> CTupleStart for (A, B) {
    type F = A;
    type B = Tuple1<B>;

    fn split_tuple_start((a, b): (A, B)) -> (A, Tuple1<B>) {
        (a, Tuple1(b))
    }

    fn join_tuple_start(a: A, b: Tuple1<B>) -> (A, B) {
        (a, b.0)
    }
}

impl<A, B, C> CTupleStart for (A, B, C) {
    type F = A;
    type B = (B, C);

    fn split_tuple_start((a, b, c): (A, B, C)) -> (A, (B, C)) {
        (a, (b, c))
    }

    fn join_tuple_start(a: A, (b, c): (B, C)) -> (A, B, C) {
        (a, b, c)
    }
}

pub trait TupleStart<F> {
    type B;

    fn split_tuple_start(zelf: Self) -> (F, Self::B);
    fn join_tuple_start(f: F, b: Self::B) -> Self;
}

impl<A> TupleStart<A> for A {
    type B = ();

    fn split_tuple_start(a: Self) -> (A, ()) {
        (a, ())
    }

    fn join_tuple_start(a: A, _: ()) -> A {
        a
    }
}

impl<A, B> TupleStart<A> for (A, B) {
    type B = B;

    fn split_tuple_start((a, b): (A, B)) -> (A, B) {
        (a, b)
    }

    fn join_tuple_start(a: A, b: B) -> (A, B) {
        (a, b)
    }
}

impl<A, B, C> TupleStart<A> for (A, B, C) {
    type B = (B, C);

    fn split_tuple_start((a, b, c): (A, B, C)) -> (A, (B, C)) {
        (a, (b, c))
    }

    fn join_tuple_start(a: A, (b, c): (B, C)) -> (A, B, C) {
        (a, b, c)
    }
}
