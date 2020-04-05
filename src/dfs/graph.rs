pub trait DfsGraph<N, KN> {
    fn expand(&self, n: &N) -> Vec<N>;
    fn end(&self, kn: &KN) -> bool;
    fn key_for(&self, n: &N) -> Option<KN>;

    fn keys_for(&self, ns: &Vec<N>) -> Vec<KN> {
        ns.iter().filter_map(|n| self.key_for(n)).collect()
    }
}
