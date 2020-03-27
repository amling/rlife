use std::hash::Hash;

use crate::dfs;

use dfs::graph::DfsGraph;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

fn materialize_path<N: Clone>(qs: &[Vec<(usize, N)>], idx: usize) -> Vec<N> {
    let mut qs = &qs[0..(qs.len() - 1)];
    let mut r = Vec::new();
    let mut idx = idx;
    while qs.len() > 0 {
        let &(idx2, ref n2) = &qs[qs.len() - 1][idx];
        r.push(n2.clone());
        qs = &qs[0..(qs.len() - 1)];
        idx = idx2;
    }
    r.reverse();
    r
}

fn check_cycle<N: Eq>(qs: &[Vec<(usize, N)>], idx: usize, n: &N) -> Option<usize> {
    let mut qs = &qs[0..(qs.len() - 1)];
    let mut idx = idx;
    while qs.len() > 0 {
        let &(idx2, ref n2) = &qs[qs.len() - 1][idx];
        if n == n2 {
            return Some(qs.len() - 1);
        }
        qs = &qs[0..(qs.len() - 1)];
        idx = idx2;
    }
    None
}

pub fn sbfs<N: Clone + Hash + Eq, R, GE: DfsGraph<N>, RE: DfsRes<N, R>, LE: DfsLifecycle<N, R>>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let mut qs = vec![vec![(0, n0)]];

    loop {
        let ql = qs.last().unwrap();
        if ql.len() == 0 {
            break;
        }

        let mut q2 = Vec::new();
        let mut r = re.empty();
        for (idx, &(prev_idx, ref n)) in ql.iter().enumerate() {
            for n2 in ge.expand(n) {
                if ge.end(&n2) {
                    let mut path = materialize_path(&qs, prev_idx);
                    path.push(n2.clone());
                    le.debug_end(&path);
                    r = re.reduce(r, re.map_end(path));
                    continue;
                }

                if let Some(idx) = check_cycle(&qs, prev_idx, n) {
                    let mut path = materialize_path(&qs, prev_idx);
                    let cycle = path.drain(idx..).collect();
                    le.debug_cycle(&path, &cycle);
                    r = re.reduce(r, re.map_cycle(path, cycle));
                    continue;
                }

                q2.push((idx, n2));
            }
        }
        qs.push(q2);
        let firstest = match qs.last().unwrap().first() {
            Some(&(idx, ref n)) => {
                let mut path = materialize_path(&qs, idx);
                path.push(n.clone());
                path
            },
            None => vec![],
        };
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}
