pub trait DfsGraph<N> {
    fn start(&self) -> N;
    fn expand(&self, n: &N) -> Vec<N>;
    fn end(&self, n: &N) -> bool;
}
