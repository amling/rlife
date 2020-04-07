use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

pub mod graph;
pub mod lifecycle;
pub mod res;

use crate::dfs;

use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

#[derive(Clone)]
struct Path<N: DfsNode> {
    vec: Vec<N::KN>,
    map: HashMap<<N::KN as DfsKeyNode>::HN, usize>,
}

impl<N: DfsNode> Path<N> {
    fn new() -> Self {
        Path {
            vec: Vec::new(),
            map: HashMap::new(),
        }
    }

    fn find_or_push(&mut self, kn: &N::KN) -> Option<usize> {
        let hn = kn.hash_node();
        if let Some(idx) = self.map.get(&hn) {
            return Some(*idx);
        }
        self.map.insert(hn.clone(), self.vec.len());
        self.vec.push(kn.clone());
        return None;
    }

    fn push(&mut self, kn: &N::KN) -> bool {
        return !self.find_or_push(kn).is_some();
    }

    fn pop(&mut self, kn_verify: &N::KN) {
        let kn = self.vec.pop().unwrap();
        debug_assert!(&kn == kn_verify);
        let r = self.map.remove(&kn.hash_node());
        debug_assert_eq!(Some(self.vec.len()), r);
    }
}

pub struct Tree<N>(pub N, pub TreeStatus<N>);

pub enum TreeStatus<N> {
    Unopened,
    Opened(Vec<Tree<N>>),
    Closed,
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub enum TreeSerdeProxyElement<N> {
    Unopened(N),
    Open(N),
    Close,
    Closed(N),
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub struct TreeSerdeProxy<N>(Vec<TreeSerdeProxyElement<N>>);

impl<N: Clone> Tree<N> {
    pub fn to_serde_proxy(&self) -> TreeSerdeProxy<N> {
        let mut acc = Vec::new();
        self.to_serde_proxy_aux(&mut acc);
        TreeSerdeProxy(acc)
    }

    fn to_serde_proxy_aux(&self, acc: &mut Vec<TreeSerdeProxyElement<N>>) {
        let n = self.0.clone();
        match self.1 {
            TreeStatus::Unopened => acc.push(TreeSerdeProxyElement::Unopened(n)),
            TreeStatus::Opened(ref children) => {
                acc.push(TreeSerdeProxyElement::Open(n));
                for child in children {
                    child.to_serde_proxy_aux(acc);
                }
                acc.push(TreeSerdeProxyElement::Close);
            }
            TreeStatus::Closed => acc.push(TreeSerdeProxyElement::Closed(n)),
        }
    }
}

impl<N: Clone> TreeSerdeProxy<N> {
    pub fn to_tree(&self) -> Tree<N> {
        let mut idx = 0;
        let r = self.to_tree_aux(&mut idx);
        assert_eq!(idx, self.0.len());
        r
    }

    fn to_tree_aux(&self, idx: &mut usize) -> Tree<N> {
        let first = &self.0[*idx];
        *idx += 1;
        match first {
            TreeSerdeProxyElement::Unopened(n) => Tree(n.clone(), TreeStatus::Unopened),
            TreeSerdeProxyElement::Open(n) => {
                let mut children = Vec::new();
                loop {
                    if let TreeSerdeProxyElement::Close = self.0[*idx] {
                        *idx += 1;
                        break;
                    }
                    children.push(self.to_tree_aux(idx));
                }
                Tree(n.clone(), TreeStatus::Opened(children))
            },
            TreeSerdeProxyElement::Closed(n) => Tree(n.clone(), TreeStatus::Closed),
            _ => panic!(),
        }
    }
}

pub fn sdfs<N: DfsNode, R, GE: DfsGraph<N>, RE: DfsRes<N::KN, R>, LE: DfsLifecycle<N, R>>(root: &mut Tree<N>, ge: &GE, re: &RE, le: &mut LE) {
    let stop = AtomicBool::new(false);

    loop {
        let mut unopened = Vec::new();
        {
            let mut path = Path::new();
            find_unopened(ge, &mut unopened, root, &mut path);
        }

        if unopened.len() == 0 {
            return;
        }

        for (tree, mut path) in unopened {
            let mut res = re.empty();
            dfs_single_thread(ge, re, le, &stop, tree, &mut path, &mut res, &mut |_| {});
            if !le.on_recollect_results(res) {
                break;
            }
        }
    }
}

pub fn dfs<N: DfsNode, R: Send, GE: DfsGraph<N> + Sync, RE: DfsRes<N::KN, R> + Sync, LE: DfsLifecycle<N, R> + Sync>(root: &mut Tree<N>, ge: &GE, re: &RE, le: &mut LE) {
    let mut very_longest: Option<Vec<N::KN>> = None;
    let mut first = true;

    loop {
        if collapse(root) {
            le.debug_checkpoint(root);
            return;
        }

        let mut unopened = Vec::new();
        {
            let mut path = Path::new();
            find_unopened(ge, &mut unopened, root, &mut path);
        }

        let mut results: Vec<_> = unopened.iter().map(|_| re.empty()).collect();
        let mut longests: Vec<Option<Vec<N::KN>>> = unopened.iter().map(|_| None).collect();

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

                let mut wait_ms = le.recollect_ms();
                if first {
                    wait_ms = 1000;
                    first = false;
                }
                std::thread::sleep(Duration::from_millis(wait_ms));

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
        let firstest = DfsNode::key_nodes(&firstest);
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

fn find_unopened<'a, N: DfsNode, GE: DfsGraph<N>>(ge: &GE, unopened: &mut Vec<(&'a mut Tree<N>, Path<N>)>, tree: &'a mut Tree<N>, path: &mut Path<N>) {
    let pop = match tree.0.key_node() {
        Some(kn) => {
            path.push(&kn);
            Some(kn)
        }
        None => None,
    };
    // Sigh, we have no need of all of tree, but borrow checker can't figure it out for some reason
    // if we match on &mut tree.1...
    match tree {
        Tree(_, TreeStatus::Unopened) => {
            unopened.push((tree, path.clone()));
        }
        Tree(_, TreeStatus::Opened(children)) => {
            for child in children.iter_mut() {
                find_unopened(ge, unopened, child, path);
            }
        }
        Tree(_, TreeStatus::Closed) => {
        }
    };
    if let Some(kn) = pop {
        path.pop(&kn);
    }
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

fn dfs_single_thread<N: DfsNode, R, GE: DfsGraph<N>, RE: DfsRes<N::KN, R>, LE: DfsLifecycle<N, R>>(ge: &GE, re: &RE, le: &LE, stop: &AtomicBool, t1: &mut Tree<N>, path: &mut Path<N>, r: &mut R, on_enter: &mut impl FnMut(&Vec<N::KN>)) -> bool {
    // TODO: don't overflow stack
    //
    // Without this it's possible for very successful/deep searches to overflow stack before
    // recollect.  With this it makes the tree in progress extremely bushy and fills memory.
    //
    // if depth >= 100 {
    //     // Don't overflow the stack!  Give up and reenter when the tree so far isn't represented on
    //     // the stack.
    //     return false;
    // }

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
                let pop = match n2.key_node() {
                    Some(kn2) => {
                        if ge.end(&kn2) {
                            let mut path = path.vec.clone();
                            path.push(kn2);
                            le.debug_end(&path);
                            add_result(r, re.map_end(path));
                            // could add Closed node, but doesn't affect anything
                            continue;
                        }

                        if let Some(idx) = path.find_or_push(&kn2) {
                            let (path, cycle) = ((&path.vec[0..idx]).to_vec(), (&path.vec[idx..]).to_vec());
                            le.debug_cycle(&path, &cycle, &kn2);
                            add_result(r, re.map_cycle(path, cycle, kn2));
                            continue;
                        }

                        Some(kn2)
                    },
                    None => None,
                };

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

                if let Some(kn2) = pop {
                    path.pop(&kn2);
                }
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
