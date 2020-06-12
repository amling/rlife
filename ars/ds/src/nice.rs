use std::fmt::Debug;
use std::hash::Hash;

marker_trait! {
    Nice:
    [Copy]
    [Debug]
    [Default]
    [Eq]
    [Hash]
    [Ord]
    [Send]
    [Sync]
}
