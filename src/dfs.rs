use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub mod graph;
pub mod lifecycle;
pub mod res;

use crate::dfs;

use dfs::graph::DfsGraph;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

#[derive(Clone)]
struct Path<N> {
    vec: Vec<N>,
    map: HashMap<N, usize>,
}

impl<N: Clone + Hash + Eq> Path<N> {
    fn new() -> Self {
        Path {
            vec: Vec::new(),
            map: HashMap::new(),
        }
    }

    fn find_or_push(&mut self, n: &N) -> Option<usize> {
        if let Some(idx) = self.map.get(n) {
            return Some(*idx);
        }
        self.map.insert(n.clone(), self.vec.len());
        self.vec.push(n.clone());
        return None;
    }

    fn push(&mut self, n: &N) -> bool {
        return !self.find_or_push(n).is_some();
    }

    fn pop(&mut self) {
        let n = self.vec.pop().unwrap();
        let r = self.map.remove(&n);
        assert_eq!(Some(self.vec.len()), r);
    }
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct Tree<N>(pub N, pub TreeStatus<N>);

#[derive(Deserialize)]
#[derive(Serialize)]
pub enum TreeStatus<N> {
    Unopened,
    Opened(Vec<Tree<N>>),
    Closed,
}

pub fn sdfs<N: Clone + Hash + Eq, R, GE: DfsGraph<N>, RE: DfsRes<N, R>, LE: DfsLifecycle<N, R>>(root: &mut Tree<N>, ge: &GE, re: &RE, le: &LE) {
    let stop = AtomicBool::new(false);

    let mut unopened = Vec::new();
    {
        let mut path = Path::new();
        find_unopened(&mut unopened, root, &mut path);
    }

    for (tree, mut path) in unopened {
        let mut res = re.empty();
        dfs_single_thread(ge, re, le, &stop, tree, &mut path, &mut res, &mut |_| {});
        if !le.on_recollect_results(res) {
            break;
        }
    }
}

pub fn dfs<N: Clone + Hash + Eq + Send, R: Send, GE: DfsGraph<N> + Sync, RE: DfsRes<N, R> + Sync, LE: DfsLifecycle<N, R> + Sync>(root: &mut Tree<N>, ge: &GE, re: &RE, le: &LE) {
    let mut very_longest: Option<Vec<N>> = None;

    loop {
        if collapse(root) {
            return;
        }

        let mut unopened = Vec::new();
        {
            let mut path = Path::new();
            find_unopened(&mut unopened, root, &mut path);
        }

        let mut results: Vec<_> = unopened.iter().map(|_| re.empty()).collect();
        let mut longests: Vec<Option<Vec<N>>> = unopened.iter().map(|_| None).collect();

        {
            let q = SegQueue::new();
            for tuple in unopened.into_iter().zip(results.iter_mut()).zip(longests.iter_mut()) {
                q.push(tuple);
            }

            let stop = AtomicBool::new(false);
            let stop = &stop;

            crossbeam::scope(|sc| {
                for _ in 0..le.threads() {
                    sc.spawn(|_| {
                        loop {
                            let (((tree, mut path), res), longest) = match q.pop() {
                                Result::Ok(tuple) => tuple,
                                Result::Err(PopError) => {
                                    return;
                                }
                            };

                            dfs_single_thread(ge, re, le, stop, tree, &mut path, res, &mut |path| {
                                let replace = match longest {
                                    Some(longest) => path.len() > longest.len(),
                                    None => true,
                                };
                                if replace {
                                    *longest = Some(path.clone());
                                }
                            });
                        }
                    });
                }

                std::thread::sleep(Duration::from_millis(le.recollect_ms()));

                stop.store(true, Ordering::Relaxed);
            }).unwrap();
        }

        let mut res = re.empty();
        for res1 in results {
            res = re.reduce(res, res1);
        }

        for longest in longests {
            if let Some(longest) = longest {
                let replace = match very_longest {
                    Some(ref very_longest) => longest.len() > very_longest.len(),
                    None => true,
                };
                if replace {
                    le.debug_longest(&longest);
                    very_longest = Some(longest);
                }
            }
        }

        let firstest = find_firstest(root);
        le.on_recollect_firstest(firstest);

        let cont = le.on_recollect_results(res);

        le.debug_checkpoint(root);

        if !cont {
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

fn find_unopened<'a, N: Eq + Hash + Clone>(unopened: &mut Vec<(&'a mut Tree<N>, Path<N>)>, tree: &'a mut Tree<N>, path: &mut Path<N>) {
    match tree {
        Tree(n, TreeStatus::Unopened) => {
            // I'm amazed borrow checker figures this one out.  Unfortunately it does not figure it
            // out if we match on &mut tree.1 instead...
            path.push(n);
            unopened.push((tree, path.clone()));
            path.pop();
        }
        Tree(n, TreeStatus::Opened(children)) => {
            path.push(n);
            for child in children.iter_mut() {
                find_unopened(unopened, child, path);
            }
            path.pop();
        }
        Tree(_, TreeStatus::Closed) => {
        }
    };
}

fn find_firstest<N: Clone>(tree: &Tree<N>) -> Vec<N> {
    let mut r = Vec::new();
    find_firstest_aux(tree, &mut r);
    r
}

fn find_firstest_aux<N: Clone>(tree: &Tree<N>, acc: &mut Vec<N>) -> bool {
    acc.push(tree.0.clone());
    match &tree.1 {
        TreeStatus::Unopened => {
            return true;
        }
        TreeStatus::Opened(children) => {
            for child in children {
                if find_firstest_aux(child, acc) {
                    return true;
                }
            }
        }
        TreeStatus::Closed => {
        }
    };
    acc.pop();
    false
}

fn dfs_single_thread<N: Clone + Eq + Hash, R, GE: DfsGraph<N>, RE: DfsRes<N, R>, LE: DfsLifecycle<N, R>>(ge: &GE, re: &RE, le: &LE, stop: &AtomicBool, t1: &mut Tree<N>, path: &mut Path<N>, r: &mut R, on_enter: &mut impl FnMut(&Vec<N>)) -> bool {
    le.debug_enter(&path.vec);
    on_enter(&path.vec);

    if stop.load(Ordering::Relaxed) {
        return false;
    }

    let add_result = |r: &mut R, r1| {
        let r0 = std::mem::replace(r, re.empty());
        *r = re.reduce(r0, r1);
    };

    match t1 {
        Tree(n1, s1 @ TreeStatus::Unopened) => {
            let mut finished = true;
            let mut children = Vec::new();
            for n2 in ge.expand(n1) {
                if ge.end(&n2) {
                    let mut path = path.vec.clone();
                    path.push(n2);
                    le.debug_end(&path);
                    add_result(r, re.map_end(path));
                    // could add Closed node, but doesn't affect anything
                    continue;
                }

                if let Some(idx) = path.find_or_push(&n2) {
                    let (path, cycle) = ((&path.vec[0..idx]).to_vec(), (&path.vec[idx..]).to_vec());
                    le.debug_cycle(&path, &cycle);
                    add_result(r, re.map_cycle(path, cycle));
                    continue;
                }

                let mut t2 = Tree(n2, TreeStatus::Unopened);
                if !dfs_single_thread(ge, re, le, stop, &mut t2, path, r, on_enter) {
                    finished = false;
                }
                if let TreeStatus::Closed = t2.1 {
                    // ditto, no need to save Closed nodes
                }
                else {
                    children.push(t2);
                }

                path.pop();
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
