trait DfsGraphConfig {
    type E;
    type N;

    fn start(e: Self::E) -> Self::N;
    fn expand(e: Self::E, n: &Self::N) -> Vec<Self::N>;
    fn end(e: Self::E, n: &Self::N) -> bool;
}

trait DfsGraphType {
    type N;

    fn start() -> Self::N;
    fn expand(n: &Self::N) -> Vec<Self::N>;
    fn end(n: &Self::N) -> bool;
}

impl<T: DfsGraphType> DfsGraphConfig for T {
    type E = ();
    type N = T::N;

    fn start(_e: ()) -> Self::N {
        T::start()
    }

    fn expand(_e: (), n: &Self::N) -> Vec<Self::N> {
        T::expand(n)
    }

    fn end(_e: (), n: &Self::N) -> bool {
        T::end(n)
    }
}

trait DfsGraphVtable {
    type N;

    fn start(&self) -> Self::N;
    fn expand(&self, n: &Self::N) -> Vec<Self::N>;
    fn end(&self, n: &Self::N) -> bool;
}

impl<N> DfsGraphConfig for &dyn DfsGraphVtable<N=N> {
    type E = Self;
    type N = N;

    fn start(zelf: Self) -> N {
        zelf.start()
    }

    fn expand(zelf: Self, n: &N) -> Vec<N> {
        zelf.expand(n)
    }

    fn end(zelf: Self, n: &N) -> bool {
        zelf.end(n)
    }
}
