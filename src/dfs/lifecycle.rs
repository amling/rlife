pub trait DfsLifecycleConfig {
    type E;
    type R;

    fn threads(e: Self::E) -> usize;
    fn recollect_ms(e: Self::E) -> usize;
    fn on_recollect(e: Self::E, r: Self::R) -> bool;
}

pub trait DfsLifecycleType {
    type R;

    fn threads() -> usize;
    fn recollect_ms() -> usize;
    fn on_recollect(r: Self::R) -> bool;
}

impl<T: DfsLifecycleType> DfsLifecycleConfig for T {
    type E = ();
    type R = T::R;

    fn threads(_e: ()) -> usize {
        T::threads()
    }

    fn recollect_ms(_e: ()) -> usize {
        T::recollect_ms()
    }

    fn on_recollect(_e: (), r: T::R) -> bool {
        T::on_recollect(r)
    }
}

pub trait DfsLifecycleVtable {
    type R;

    fn threads(&self) -> usize;
    fn recollect_ms(&self) -> usize;
    fn on_recollect(&self, r: Self::R) -> bool;
}

impl<R> DfsLifecycleConfig for &dyn DfsLifecycleVtable<R=R> {
    type E = Self;
    type R = R;

    fn threads(zelf: Self) -> usize {
        zelf.threads()
    }

    fn recollect_ms(zelf: Self) -> usize {
        zelf.recollect_ms()
    }

    fn on_recollect(zelf: Self, r: Self::R) -> bool {
        zelf.on_recollect(r)
    }
}
