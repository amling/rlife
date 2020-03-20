pub trait DfsLifecycleConfig {
    type E;
    type R;

    fn threads(e: &Self::E) -> usize;
    fn recollect_ms(e: &Self::E) -> u64;
    fn on_recollect(e: &Self::E, r: Self::R) -> bool;
}

pub trait DfsLifecycleType {
    type R;

    fn threads() -> usize;
    fn recollect_ms() -> u64;
    fn on_recollect(r: Self::R) -> bool;
}

impl<T: DfsLifecycleType> DfsLifecycleConfig for T {
    type E = ();
    type R = T::R;

    fn threads(_: &()) -> usize {
        T::threads()
    }

    fn recollect_ms(_: &()) -> u64 {
        T::recollect_ms()
    }

    fn on_recollect(_: &(), r: T::R) -> bool {
        T::on_recollect(r)
    }
}

pub trait DfsLifecycleVtable {
    type R;

    fn threads(&self) -> usize;
    fn recollect_ms(&self) -> u64;
    fn on_recollect(&self, r: Self::R) -> bool;
}

impl<R> DfsLifecycleConfig for &dyn DfsLifecycleVtable<R=R> {
    type E = Self;
    type R = R;

    fn threads(zelf: &Self) -> usize {
        zelf.threads()
    }

    fn recollect_ms(zelf: &Self) -> u64 {
        zelf.recollect_ms()
    }

    fn on_recollect(zelf: &Self, r: Self::R) -> bool {
        zelf.on_recollect(r)
    }
}
