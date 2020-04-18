use crate::dfs;

use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
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

fn check_cycle<N: DfsNode>(qs: &[Vec<(usize, N)>], idx: usize, kn: &N::KN) -> Option<usize> {
    let mut qs = &qs[0..(qs.len() - 1)];
    let mut idx = idx;
    while qs.len() > 0 {
        let &(idx2, ref n2) = &qs[qs.len() - 1][idx];
        if let Some(kn2) = n2.key_node() {
            if kn.hash_node() == kn2.hash_node() {
                return Some(qs.len() - 1);
            }
        }
        qs = &qs[0..(qs.len() - 1)];
        idx = idx2;
    }
    None
}

pub fn bfs<N: DfsNode, R, GE: DfsGraph<N>, RE: DfsRes<N::KN, R>, LE: DfsLifecycle<N, R>>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let mut qs = vec![vec![(0, n0)]];
    let mut ttl = 1;

    loop {
        let ql = qs.last().unwrap();
        if ql.len() == 0 {
            break;
        }

        let mut q2 = Vec::new();
        let mut r = re.empty();
        for (idx, &(prev_idx, ref n)) in ql.iter().enumerate() {
            for n2 in ge.expand(n) {
                if let Some(kn2) = n2.key_node() {
                    if ge.end(&kn2) {
                        let path = materialize_path(&qs, prev_idx);
                        let mut path = DfsNode::key_nodes(&path);
                        path.push(kn2);
                        le.debug_end(&path);
                        r = re.reduce(r, re.map_end(path));
                        continue;
                    }

                    if let Some(idx) = check_cycle(&qs, prev_idx, &kn2) {
                        let mut path = materialize_path(&qs, prev_idx);
                        let cycle = path.drain(idx..).collect();
                        let path = DfsNode::key_nodes(&path);
                        let cycle = DfsNode::key_nodes(&cycle);
                        le.debug_cycle(&path, &cycle, &kn2);
                        r = re.reduce(r, re.map_cycle(path, cycle, kn2));
                        continue;
                    }
                }

                q2.push((idx, n2));
            }
        }
        ttl += q2.len();
        eprintln!("Completed BFS step ql {} => q2 {} (total {})", ql.len(), q2.len(), ttl);
        qs.push(q2);
        let firstest = match qs.last().unwrap().first() {
            Some(&(idx, ref n)) => {
                let mut path = materialize_path(&qs, idx);
                path.push(n.clone());
                path
            },
            None => vec![],
        };
        let firstest = DfsNode::key_nodes(&firstest);
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}
