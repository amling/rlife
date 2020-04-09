pub trait TupleEnd {
    type F;
    type B;

    fn split_tuple_end(zelf: Self) -> (Self::F, Self::B);
    fn join_tuple_end(f: Self::F, b: Self::B) -> Self;
}

macro_rules! tuple_end_impl {
    ([$($head:ident: $HEAD: ident,)*][$tail:ident: $TAIL:ident]) => {
        #[allow(unused_parens)]
        impl<$($HEAD,)* $TAIL> TupleEnd for ($($HEAD,)* $TAIL,) {
            type F = ($($HEAD,)*);
            type B = $TAIL;

            fn split_tuple_end(($($head,)* $tail,): ($($HEAD,)* $TAIL,)) -> (($($HEAD,)*), $TAIL) {
                (($($head,)*), $tail)
            }

            fn join_tuple_end(($($head,)*): ($($HEAD,)*), $tail: $TAIL) -> ($($HEAD,)* $TAIL,) {
                ($($head,)* $tail,)
            }
        }
    };
}

tuple_end_impl!([][z: Z]);
tuple_end_impl!([a: A,][z: Z]);
tuple_end_impl!([a: A, b: B,][z: Z]);
tuple_end_impl!([a: A, b: B, c: C,][z: Z]);
tuple_end_impl!([a: A, b: B, c: C, d: D,][z: Z]);
tuple_end_impl!([a: A, b: B, c: C, d: D, e: E,][z: Z]);
tuple_end_impl!([a: A, b: B, c: C, d: D, e: E, f: F,][z: Z]);
tuple_end_impl!([a: A, b: B, c: C, d: D, e: E, f: F, g: G,][z: Z]);

pub trait TupleStart {
    type F;
    type B;

    fn split_tuple_start(zelf: Self) -> (Self::F, Self::B);
    fn join_tuple_start(f: Self::F, b: Self::B) -> Self;
}

macro_rules! tuple_start_impl {
    ([$head:ident: $HEAD:ident][$($tail:ident: $TAIL:ident,)*]) => {
        impl<$HEAD, $($TAIL,)*> TupleStart for ($HEAD, $($TAIL,)*) {
            type F = $HEAD;
            type B = ($($TAIL,)*);

            fn split_tuple_start(($head, $($tail,)*): ($HEAD, $($TAIL,)*)) -> ($HEAD, ($($TAIL,)*)) {
                ($head, ($($tail,)*))
            }

            fn join_tuple_start($head: $HEAD, ($($tail,)*): ($($TAIL,)*)) -> ($HEAD, $($TAIL,)*) {
                ($head, $($tail,)*)
            }
        }
    }
}

tuple_start_impl!([z: Z][]);
tuple_start_impl!([z: Z][a: A,]);
tuple_start_impl!([z: Z][a: A, b: B,]);
tuple_start_impl!([z: Z][a: A, b: B, c: C,]);
tuple_start_impl!([z: Z][a: A, b: B, c: C, d: D,]);
tuple_start_impl!([z: Z][a: A, b: B, c: C, d: D, e: E,]);
tuple_start_impl!([z: Z][a: A, b: B, c: C, d: D, e: E, f: F,]);

// This is to work around rust's issues with colliding impls.  We'd like to let people e.g.
// implement some trait of theirs for isize but then also for all tuples by recursion.  Since we're
// in another crate rust complains that someone could implement tuple for isize which of course we
// will never do.  We use this to let people define their own marker trait for things we've
// actually implemented Tuple* for.
#[macro_export]
macro_rules! is_tuple_trait {
    ($t:ident) => {
        pub trait $t { }
        impl<A> $t for (A,) { }
        impl<A, B> $t for (A, B) { }
        impl<A, B, C> $t for (A, B, C) { }
        impl<A, B, C, D> $t for (A, B, C, D) { }
        impl<A, B, C, D, E> $t for (A, B, C, D, E) { }
        impl<A, B, C, D, E, F> $t for (A, B, C, D, E, F) { }
        impl<A, B, C, D, E, F, G> $t for (A, B, C, D, E, F, G) { }
    }
}
