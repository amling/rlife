trait DfsReportConfig {
    type E;
    type R;

    fn recollect_ms(e: Self::E) -> usize;
    fn on_recollect(e: Self::E, r: Self::R) -> bool;
}

trait DfsReportType {
    type R;

    fn recollect_ms() -> usize;
    fn on_recollect(r: Self::R) -> bool;
}

impl<T: DfsReportType> DfsReportConfig for T {
    type E = ();
    type R = T::R;

    fn recollect_ms(_e: ()) -> usize {
        T::recollect_ms()
    }

    fn on_recollect(_e: (), r: T::R) -> bool {
        T::on_recollect(r)
    }
}

trait DfsReportVtable {
    type R;

    fn recollect_ms(&self) -> usize;
    fn on_recollect(&self, r: Self::R) -> bool;
}

impl<R> DfsReportConfig for &dyn DfsReportVtable<R=R> {
    type E = Self;
    type R = R;

    fn recollect_ms(zelf: Self) -> usize {
        zelf.recollect_ms()
    }

    fn on_recollect(zelf: Self, r: Self::R) -> bool {
        zelf.on_recollect(r)
    }
}
