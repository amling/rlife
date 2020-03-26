pub trait DfsGraph<N> {
    fn expand(&self, n: &N) -> Vec<N>;
    fn end(&self, n: &N) -> bool;
}
