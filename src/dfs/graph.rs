use std::fmt::Debug;
use std::hash::Hash;

marker_trait! {
    DfsNodeCommon:
    [Clone]
    [Debug]
    [Default]
    [Eq]
    [Hash]
    [Send]
    [Sync]
}

pub trait DfsNode: DfsNodeCommon {
    type KN: DfsKeyNode;

    fn key_node(&self) -> Option<Self::KN>;

    fn key_nodes(v: &Vec<Self>) -> Vec<Self::KN> {
        v.iter().filter_map(|n| n.key_node()).collect()
    }
}

pub trait DfsKeyNode: DfsNodeCommon {
    type HN: DfsNodeCommon;

    fn hash_node(&self) -> Option<Self::HN>;
}

pub trait DfsGraph<N: DfsNode> {
    fn expand(&self, n: &N) -> Vec<N>;
    fn end(&self, kn: &N::KN) -> Option<&str>;
}
