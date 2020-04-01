use ars_aa::zmodule::ZModule;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

// Finds generators for the space of cycle weights reachable from the given node.
pub fn find_cycle_generators<N: Hash + Clone + Eq, R: ZModule + Clone>(links: &HashMap<N, HashSet<(N, R)>>, n: N) -> Vec<R> {
    ars_aa::misc::find_cycle_weight_generators(links, n)
}

// Finds the set of connected nodes and the weight of some arbitrary path to them
pub fn find_connected<N: Hash + Clone + Eq, R: ZModule + Clone>(links: &HashMap<N, HashSet<(N, R)>>, n: N) -> HashMap<N, R> {
    ars_aa::misc::find_connected_weights(links, n)
}
