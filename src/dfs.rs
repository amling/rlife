trait DfsConfig {
    type E;
    type N;

    fn start(e: Self::E) -> Self::N;
    fn end(e: Self::E, n: &Self::N) -> bool;
}

trait DfsType {
    type N;

    fn start() -> Self::N;
    fn end(n: &Self::N) -> bool;
}

impl<N, T: DfsType<N=N>> DfsConfig for T {
    type E = ();
    type N = N;

    fn start(_e: ()) -> N {
        T::start()
    }

    fn end(_e: (), n: &N) -> bool {
        T::end(n)
    }
}

trait DfsVtable {
    type N;

    fn start(&self) -> Self::N;
    fn end(&self, n: &Self::N) -> bool;
}

impl<N> DfsConfig for &dyn DfsVtable<N=N> {
    type E = Self;
    type N = N;

    fn start(zelf: Self) -> N {
        zelf.start()
    }

    fn end(zelf: Self, n: &N) -> bool {
        zelf.end(n)
    }
}
