mod graph;
mod lifecycle;
mod res;

use crate::dfs;

use dfs::graph::DfsGraphConfig;
use dfs::lifecycle::DfsLifecycleConfig;
use dfs::res::DfsResConfig;

fn dfs<N, R, GE, GC: DfsGraphConfig<E=GE, N=N>, RE, RC: DfsResConfig<E=RE, N=N, R=R>, LE, LC: DfsLifecycleConfig<E=LE, R=R>>(ge: GE, re: RE, le: LE) {
}
