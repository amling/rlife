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

macro_rules! ctuple_end_impl {
    ([$($head:tt)*][$($HEAD:tt)*][$tail:ident][$TAIL:ident]) => {
        impl<$($HEAD)*, $TAIL> CTupleEnd for ($($HEAD)*, $TAIL) {
            type F = ($($HEAD)*);
            type B = $TAIL;

            fn split_tuple_end(($($head)*, $tail): ($($HEAD)*, $TAIL)) -> (($($HEAD)*), $TAIL) {
                (($($head)*), $tail)
            }

            fn join_tuple_end(($($head)*): ($($HEAD)*), $tail: $TAIL) -> ($($HEAD)*, $TAIL) {
                ($($head)*, $tail)
            }
        }
    }
}

ctuple_end_impl!([a, b][A, B][z][Z]);
ctuple_end_impl!([a, b, c][A, B, C][z][Z]);
ctuple_end_impl!([a, b, c, d][A, B, C, D][z][Z]);
ctuple_end_impl!([a, b, c, d, e][A, B, C, D, E][z][Z]);
ctuple_end_impl!([a, b, c, d, e, f][A, B, C, D, E, F][z][Z]);

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

macro_rules! tuple_end_impl {
    ([$($head:tt)*][$($HEAD:tt)*][$tail:ident][$TAIL:ident]) => {
        impl<$($HEAD)*, $TAIL> TupleEnd<$TAIL> for ($($HEAD)*, $TAIL) {
            type F = ($($HEAD)*);

            fn split_tuple_end(($($head)*, $tail): ($($HEAD)*, $TAIL)) -> (($($HEAD)*), $TAIL) {
                (($($head)*), $tail)
            }

            fn join_tuple_end(($($head)*): ($($HEAD)*), $tail: $TAIL) -> ($($HEAD)*, $TAIL) {
                ($($head)*, $tail)
            }
        }
    }
}

tuple_end_impl!([a, b][A, B][z][Z]);
tuple_end_impl!([a, b, c][A, B, C][z][Z]);
tuple_end_impl!([a, b, c, d][A, B, C, D][z][Z]);
tuple_end_impl!([a, b, c, d, e][A, B, C, D, E][z][Z]);
tuple_end_impl!([a, b, c, d, e, f][A, B, C, D, E, F][z][Z]);

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

macro_rules! ctuple_start_impl {
    ([$head:ident][$HEAD:ident][$($tail:tt)*][$($TAIL:tt)*]) => {
        impl<$HEAD, $($TAIL)*> CTupleStart for ($HEAD, $($TAIL)*) {
            type F = $HEAD;
            type B = ($($TAIL)*);

            fn split_tuple_start(($head, $($tail)*): ($HEAD, $($TAIL)*)) -> ($HEAD, ($($TAIL)*)) {
                ($head, ($($tail)*))
            }

            fn join_tuple_start($head: $HEAD, ($($tail)*): ($($TAIL)*)) -> ($HEAD, $($TAIL)*) {
                ($head, $($tail)*)
            }
        }
    }
}

ctuple_start_impl!([z][Z][a, b][A, B]);
ctuple_start_impl!([z][Z][a, b, c][A, B, C]);
ctuple_start_impl!([z][Z][a, b, c, d][A, B, C, D]);
ctuple_start_impl!([z][Z][a, b, c, d, e][A, B, C, D, E]);
ctuple_start_impl!([z][Z][a, b, c, d, e, f][A, B, C, D, E, F]);

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

macro_rules! tuple_start_impl {
    ([$head:ident][$HEAD:ident][$($tail:tt)*][$($TAIL:tt)*]) => {
        impl<$HEAD, $($TAIL)*> TupleStart<$HEAD> for ($HEAD, $($TAIL)*) {
            type B = ($($TAIL)*);

            fn split_tuple_start(($head, $($tail)*): ($HEAD, $($TAIL)*)) -> ($HEAD, ($($TAIL)*)) {
                ($head, ($($tail)*))
            }

            fn join_tuple_start($head: $HEAD, ($($tail)*): ($($TAIL)*)) -> ($HEAD, $($TAIL)*) {
                ($head, $($tail)*)
            }
        }
    }
}

tuple_start_impl!([z][Z][a, b][A, B]);
tuple_start_impl!([z][Z][a, b, c][A, B, C]);
tuple_start_impl!([z][Z][a, b, c, d][A, B, C, D]);
tuple_start_impl!([z][Z][a, b, c, d, e][A, B, C, D, E]);
tuple_start_impl!([z][Z][a, b, c, d, e, f][A, B, C, D, E, F]);

// This is to work around rust's issues with colliding impls.  We'd like to let people e.g.
// implement some trait of theirs for isize but then also for all tuples by recursion.  Since we're
// in another crate rust complains that someone could implement tuple for isize which of course we
// will never do.  We use this to let people define their own marker trait for things we've
// actually implemented CTuple* for.
#[macro_export]
macro_rules! is_tuple_trait {
    ($t:ident) => {
        pub trait $t { }
        impl<A> $t for $crate::tuple::Tuple1<A> { }
        impl<A, B> $t for (A, B) { }
        impl<A, B, C> $t for (A, B, C) { }
        impl<A, B, C, D> $t for (A, B, C, D) { }
        impl<A, B, C, D, E> $t for (A, B, C, D, E) { }
        impl<A, B, C, D, E, F> $t for (A, B, C, D, E, F) { }
        impl<A, B, C, D, E, F, G> $t for (A, B, C, D, E, F, G) { }
    }
}
