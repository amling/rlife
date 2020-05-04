use crossbeam::queue::PopError;
use crossbeam::queue::SegQueue;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Condvar;
use std::sync::Mutex;
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
pub struct Path<N: DfsNode> {
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

    pub fn from_vec(v: Vec<N::KN>) -> Self {
        let map = v.iter().enumerate().map(|(idx, kn)| (kn.hash_node(), idx)).collect();
        Path {
            vec: v,
            map: map,
        }
    }

    pub fn find_or_push(&mut self, kn: &N::KN) -> Option<usize> {
        let hn = kn.hash_node();
        if let Some(idx) = self.map.get(&hn) {
            return Some(*idx);
        }
        self.map.insert(hn.clone(), self.vec.len());
        self.vec.push(kn.clone());
        return None;
    }

    pub fn push(&mut self, kn: &N::KN) -> bool {
        return !self.find_or_push(kn).is_some();
    }

    pub fn pop(&mut self, kn_verify: &N::KN) {
        let kn = self.vec.pop().unwrap();
        debug_assert!(&kn == kn_verify);
        let r = self.map.remove(&kn.hash_node());
        debug_assert_eq!(Some(self.vec.len()), r);
    }
}

pub struct Tree<N>(pub N, pub TreeStatus<N>);

impl<N> Tree<N> {
    pub fn map<N2>(self, f: &mut impl FnMut(N) -> N2) -> Tree<N2> {
        Tree(f(self.0), self.1.map(f))
    }

    pub fn as_ref(&self) -> Tree<&N> {
        Tree(&self.0, self.1.as_ref())
    }
}

pub enum TreeStatus<N> {
    Unopened,
    Opened(Vec<Tree<N>>),
    Closed,
}

impl<N> TreeStatus<N> {
    pub fn map<N2>(self, f: &mut impl FnMut(N) -> N2) -> TreeStatus<N2> {
        match self {
            TreeStatus::Unopened => TreeStatus::Unopened,
            TreeStatus::Opened(children) => TreeStatus::Opened(children.into_iter().map(|t| t.map(f)).collect()),
            TreeStatus::Closed => TreeStatus::Closed,
        }
    }

    fn as_ref(&self) -> TreeStatus<&N> {
        match self {
            TreeStatus::Unopened => TreeStatus::Unopened,
            TreeStatus::Opened(children) => TreeStatus::Opened(children.iter().map(|n| n.as_ref()).collect()),
            TreeStatus::Closed => TreeStatus::Closed,
        }
    }
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
eprintln!("to proxy cts {:?}", self.proxy_cts());
        let mut acc = Vec::new();
        self.to_serde_proxy_aux(&mut acc);
        TreeSerdeProxy(acc)
    }

    pub fn proxy_cts(&self) -> (usize, usize, usize, usize) {
        let mut acc = (0, 0, 0, 0);
        self.proxy_cts_aux(&mut acc);
        acc
    }

    fn proxy_cts_aux(&self, acc: &mut (usize, usize, usize, usize)) {
        match self.1 {
            TreeStatus::Unopened => {
                acc.0 += 1;
            }
            TreeStatus::Opened(ref children) => {
                acc.1 += 1;
                for child in children {
                    child.proxy_cts_aux(acc);
                }
                acc.2 += 1;
            }
            TreeStatus::Closed => {
                acc.3 += 1;
            }
        }
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
eprintln!("to tree cts {:?}", r.proxy_cts());
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

pub fn sdfs<N: DfsNode, GE: DfsGraph<N>, LE: DfsLifecycle<N>>(root: &mut Tree<N>, ge: &GE, le: &mut LE) {
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
            let mut res = DfsRes::new();
            dfs_single_thread(ge, le, tree, &mut path, &mut res, &mut |_| true);
            if !le.on_recollect_results(res) {
                break;
            }
        }
    }
}

pub fn dfs<N: DfsNode, GE: DfsGraph<N> + Sync, LE: DfsLifecycle<N> + Sync>(root: &mut Tree<N>, ge: &GE, le: &mut LE) {
    let mut very_longest: Option<Vec<N::KN>> = None;

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

        let mut results: Vec<_> = unopened.iter().map(|_| DfsRes::new()).collect();
        let mut longests: Vec<Option<Vec<N::KN>>> = unopened.iter().map(|_| None).collect();

        {
            let q = SegQueue::new();
            for tuple in unopened.into_iter().zip(results.iter_mut()).zip(longests.iter_mut()) {
                q.push(tuple);
            }

            let stop = AtomicBool::new(false);
            let stopped = (Mutex::new(false), Condvar::new());

            crossbeam::scope(|sc| {
                for _ in 0..le.threads() {
                    sc.spawn(|_| {
                        loop {
                            let (((tree, mut path), res), longest) = match q.pop() {
                                Result::Ok(tuple) => tuple,
                                Result::Err(PopError) => {
                                    // we're out of work, force recollect
                                    let mut mg = stopped.0.lock().unwrap();
                                    *mg = true;

                                    // It's important that notify occur strictly after having
                                    // locked to ensure write is visible to main thread when it
                                    // wakes up from this notify.
                                    stopped.1.notify_all();

                                    return;
                                }
                            };

                            dfs_single_thread(ge, le, tree, &mut path, res, &mut |path| {
                                let replace = match longest {
                                    Some(longest) => path.len() > longest.len(),
                                    None => true,
                                };
                                if replace {
                                    *longest = Some(path.clone());
                                }
                                !stop.load(Ordering::Relaxed)
                            });

                            // Previously dfs_single_thread would check stop before doing anything,
                            // but now it always expands the first level.  It's better to skip
                            // pointlessly calling it for every queue entry anyway.  We do however
                            // always expand at least one work unit if we can find one to ensure
                            // progress.
                            if stop.load(Ordering::Relaxed) {
                                return;
                            }
                        }
                    });
                }

                // Wait up to recollect time for some thread to decide we're done.  This is subject
                // to spurious wakeups but we don't have wait_timeout_while yet (rage) and it's not
                // a big deal to have occasional spurious wakeups.
                let mg = stopped.0.lock().unwrap();
                let (mg, _) = stopped.1.wait_timeout(mg, Duration::from_millis(le.recollect_ms())).unwrap();
                // don't care, go away
                std::mem::drop(mg);

                // tell everyone else to bail
                stop.store(true, Ordering::Relaxed);
            }).unwrap();
        }

        let mut res = DfsRes::new();
        for mut res1 in results {
            res.append(&mut res1);
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
        let firstest = (DfsNode::key_nodes(&firstest), firstest.last().unwrap().clone());
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

fn dfs_single_thread<N: DfsNode, GE: DfsGraph<N>, LE: DfsLifecycle<N>>(ge: &GE, le: &LE, t1: &mut Tree<N>, path: &mut Path<N>, r: &mut DfsRes<N::KN>, on_enter: &mut impl FnMut(&Vec<N::KN>) -> bool) -> bool {
    // Unpack to just the node
    let n1 = t1.0.clone();
    match t1.1 {
        TreeStatus::Unopened => {
            // ok
        },
        _ => panic!(),
    }
    let n2s = ge.expand(&n1).into_iter();

    // hard-code none for KN because we actually don't want to pop the caller-provided KN
    // corresponding to n1 off the path (if there is one)
    let mut stack = vec![(n1, None, n2s)];
    'top: loop {
        // invariants:
        //
        // we're looking for next node to enter
        //
        // path and stack "match"

        let n1 = match stack.last_mut() {
            Some(last) => {
                match last.2.next() {
                    // found (and pulled) another unopened node, continue from here
                    Some(n1) => n1,
                    None => {
                        // no more children, you're done, move up to parent and keep looking
                        if let Some(kn) = &last.1 {
                            path.pop(kn);
                        }
                        stack.pop();
                        continue 'top;
                    },
                }
            },
            None => {
                // yay, we finished everything
                t1.1 = TreeStatus::Closed;
                return true;
            },
        };

        // found a node to enter, let's put it on the stack
        let kn1 = n1.key_node();
        if let Some(kn1) = &kn1 {
            if ge.end(kn1) {
                let mut path = path.vec.clone();
                path.push(kn1.clone());
                le.debug_end(&path);
                r.add_end(path);
                continue 'top;
            }

            if let Some(idx) = path.find_or_push(kn1) {
                let (path, cycle) = ((&path.vec[0..idx]).to_vec(), (&path.vec[idx..]).to_vec());
                le.debug_cycle(&path, &cycle, kn1);
                r.add_cycle(path, cycle, kn1.clone());
                continue 'top;
            }
        }

        // this is the point where we'd be [re]entering in the old recursive version

        le.debug_enter((&path.vec, &n1));
        let stop = !on_enter(&path.vec);

        if stop {
            let mut tr = Tree(n1, TreeStatus::Unopened);
            for (n, _kn, children2) in stack.into_iter().rev() {
                let mut children = vec![tr];
                for n2 in children2 {
                    children.push(Tree(n2, TreeStatus::Unopened));
                }
                tr = Tree(n, TreeStatus::Opened(children));
            }
            debug_assert_eq!(t1.0, tr.0);
            *t1 = tr;
            return false;
        }

        let n2s = ge.expand(&n1).into_iter();
        stack.push((n1, kn1, n2s));
    }
}
