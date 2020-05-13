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

    fn hash_node<'a>(&'a self, path: impl Iterator<Item=&'a Self>) -> Option<Self::HN>;
}

pub trait DfsGraph<N: DfsNode> {
    fn expand<'a>(&'a self, n: &'a N, path: impl Iterator<Item=&'a N::KN>) -> Vec<N>;
    fn end<'a>(&'a self, kn: &'a N::KN, path: impl Iterator<Item=&'a N::KN>) -> Option<&'static str>;
}
