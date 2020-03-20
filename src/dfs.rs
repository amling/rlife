use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

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

pub fn dfs<N: Clone + Hash + Eq + Send, R: Send, GE: Send + Sync + Copy, GC: DfsGraphConfig<E=GE, N=N>, RE: Send + Sync + Copy, RC: DfsResConfig<E=RE, N=N, R=R>, LE: Sync + Copy, LC: DfsLifecycleConfig<E=LE, R=R>>(ge: GE, re: RE, le: LE) {
    let n0 = GC::start(ge);
    let mut root = Tree(n0, TreeStatus::Unopened);

    loop {
        if collapse(&mut root) {
            return;
        }

        let mut unopened = Vec::new();
        {
            let mut already = HashSet::new();
            find_unopened(&mut unopened, &mut root, &mut already);
        }

        let mut results: Vec<_> = unopened.iter().map(|_| RC::empty(re)).collect();

        {
            let q = SegQueue::new();
            for pair in unopened.into_iter().zip(results.iter_mut()) {
                q.push(pair);
            }

            let stop = AtomicBool::new(false);
            let stop = &stop;

            crossbeam::scope(|sc| {
                for _ in 0..LC::threads(le) {
                    sc.spawn(|_| {
                        loop {
                            let ((tree, mut already), res) = match q.pop() {
                                Result::Ok(pair) => pair,
                                Result::Err(PopError) => {
                                    return;
                                }
                            };

                            dfs_single_thread::<N, R, GE, GC, RE, RC>(ge, re, stop, tree, &mut already, res);
                        }
                    });
                }

                std::thread::sleep(Duration::from_millis(LC::recollect_ms(le)));

                stop.store(true, Ordering::Relaxed);
            });
        }

        let mut res = RC::empty(re);
        for res1 in results {
            res = RC::reduce(re, res, res1);
        }

        if !LC::on_recollect(le, res) {
            return;
        }
    }
}

fn collapse<N>(tree: &mut Tree<N>) -> bool {
    let status = &mut tree.1;
    match status {
        TreeStatus::Unopened => false,
        TreeStatus::Opened(children) => {
            let mut finished = true;
            for child in children.iter_mut() {
                if !collapse(child) {
                    finished = false;
                }
            }
            if finished {
                *status = TreeStatus::Closed;
            }
            finished
        },
        TreeStatus::Closed => true,
    }
}

fn find_unopened<'a, N: Eq + Hash + Clone>(unopened: &mut Vec<(&'a mut Tree<N>, HashSet<N>)>, tree: &'a mut Tree<N>, already: &mut HashSet<N>) {
    match tree {
        Tree(_, TreeStatus::Unopened) => {
            // I'm amazed borrow checker figures this one out.  Unfortunately it does not figure it
            // out if we match on &mut tree.1 instead...
            unopened.push((tree, already.clone()));
        }
        Tree(n, TreeStatus::Opened(children)) => {
            already.insert(n.clone());
            for child in children.iter_mut() {
                find_unopened(unopened, child, already);
            }
            already.remove(n);
        }
        Tree(_, TreeStatus::Closed) => {
        }
    };
}

fn dfs_single_thread<N: Clone + Eq + Hash, R, GE: Copy, GC: DfsGraphConfig<E=GE, N=N>, RE: Copy, RC: DfsResConfig<E=RE, N=N, R=R>>(ge: GE, re: RE, stop: &AtomicBool, t1: &mut Tree<N>, already: &mut HashSet<N>, r: &mut R) -> bool {
    if stop.load(Ordering::Relaxed) {
        return false;
    }

    match t1 {
        Tree(n1, s1 @ TreeStatus::Unopened) => {
            let mut finished = true;
            let mut children = Vec::new();
            for n2 in GC::expand(ge, n1) {
                if GC::end(ge, &n2) {
                    unimplemented!();
                }

                if already.insert(n2.clone()) {
                    unimplemented!();
                }
                let mut t2 = Tree(n2, TreeStatus::Unopened);
                if !dfs_single_thread::<N, R, GE, GC, RE, RC>(ge, re, stop, &mut t2, already, r) {
                    finished = false;
                }
                already.remove(&t2.0);
                children.push(t2);
            }
            *s1 = match finished {
                true => TreeStatus::Closed,
                false => TreeStatus::Opened(children),
            };
            return finished;
        }
        _ => panic!()
    };
}
