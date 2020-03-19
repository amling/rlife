trait DfsResConfig {
    type E;
    type N;
    type R;

    fn empty(e: Self::E) -> Self::R;
    fn map_cycle(e: Self::E, _path: Vec<Self::N>, _cycle: Vec<Self::N>) -> Self::R {
        Self::empty(e)
    }
    fn map_end(e: Self::E, path: Vec<Self::N>) -> Self::R;
    fn reduce(e: Self::E, r1: Self::R, r2: Self::R);
}

trait DfsResType {
    type N;
    type R;

    fn empty() -> Self::R;
    fn map_cycle(path: Vec<Self::N>, cycle: Vec<Self::N>) -> Self::R;
    fn map_end(path: Vec<Self::N>) -> Self::R;
    fn reduce(r1: Self::R, r2: Self::R);
}

impl<T: DfsResType> DfsResConfig for T {
    type E = ();
    type N = T::N;
    type R = T::R;

    fn empty(_e: Self::E) -> Self::R {
        T::empty()
    }

    fn map_cycle(_e: Self::E, path: Vec<Self::N>, cycle: Vec<Self::N>) -> Self::R {
        T::map_cycle(path, cycle)
    }

    fn map_end(_e: Self::E, path: Vec<Self::N>) -> Self::R {
        T::map_end(path)
    }

    fn reduce(_e: Self::E, r1: Self::R, r2: Self::R) {
        T::reduce(r1, r2)
    }
}

trait DfsResVtable {
    type N;
    type R;

    fn empty(&self) -> Self::R;
    fn map_cycle(&self, path: Vec<Self::N>, cycle: Vec<Self::N>) -> Self::R;
    fn map_end(&self, path: Vec<Self::N>) -> Self::R;
    fn reduce(&self, r1: Self::R, r2: Self::R);
}

impl<N, R> DfsResConfig for &dyn DfsResVtable<N=N, R=R> {
    type E = Self;
    type N = N;
    type R = R;

    fn empty(zelf: Self) -> R {
        zelf.empty()
    }

    fn map_cycle(zelf: Self, path: Vec<N>, cycle: Vec<N>) -> R {
        zelf.map_cycle(path, cycle)
    }

    fn map_end(zelf: Self, path: Vec<N>) -> R {
        zelf.map_end(path)
    }

    fn reduce(zelf: Self, r1: R, r2: R) {
        zelf.reduce(r1, r2)
    }
}
