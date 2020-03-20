pub trait DfsLifecycle<N, R> {
    fn threads(&self) -> usize;
    fn recollect_ms(&self) -> u64;
    fn on_recollect(&self, r: R) -> bool;
    fn debug_enter(&self, _path: &Vec<N>) {
    }
}
