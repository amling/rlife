use crossbeam::queue::SegQueue;

mod graph;
mod lifecycle;
mod res;

use crate::dfs;

use dfs::graph::DfsGraphConfig;
use dfs::lifecycle::DfsLifecycleConfig;
use dfs::res::DfsResConfig;

struct Tree<N>(N, TreeStatus<N>);

enum TreeStatus<N> {
    Unopened,
    Opened(Vec<Tree<N>>),
    Closed,
}

pub fn dfs<N, R, GE, GC: DfsGraphConfig<E=GE, N=N>, RE: Copy, RC: DfsResConfig<E=RE, N=N, R=R>, LE: Copy, LC: DfsLifecycleConfig<E=LE, R=R>>(ge: GE, re: RE, le: LE) {
    let n0 = GC::start(ge);
    let mut root = Tree(n0, TreeStatus::Unopened);

    loop {
        let mut unopened = Vec::new();
        find_unopened(&mut unopened, &mut root);

        let mut results: Vec<_> = unopened.iter().map(|_| RC::empty(re)).collect();
        let q = SegQueue::new();
        for pair in unopened.into_iter().zip(results.iter_mut()) {
            q.push(pair);
        }

        crossbeam::scope(|sc| {
            for _ in 0..LC::threads(le) {
                sc.spawn(|_| {
                });
            }
        });
    }
}

fn find_unopened<'a, N>(unopened: &mut Vec<&'a mut Tree<N>>, tree: &'a mut Tree<N>) {
    match tree {
        Tree(_, TreeStatus::Unopened) => {
            // I'm amazed borrow checker figures this one out.  Unfortunately it does not figure it
            // out if we match on &mut tree.1 instead...
            unopened.push(tree);
        }
        Tree(_, TreeStatus::Opened(children)) => {
            for child in children.iter_mut() {
                find_unopened(unopened, child);
            }
        }
        Tree(_, TreeStatus::Closed) => {
        }
    };
}
