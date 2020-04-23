use std::collections::VecDeque;

use crate::bfs;
use crate::dfs;

use bfs::kn_pile::KnPile;
use dfs::graph::DfsGraph;
use dfs::graph::DfsKeyNode;
use dfs::graph::DfsNode;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

pub fn bfs2<N: DfsNode, R, GE: DfsGraph<N>, RE: DfsRes<N::KN, R>, LE: DfsLifecycle<N, R>>(n0: N, ge: &GE, re: &RE, le: &mut LE) {
    let mut kns;
    let mut q;
    let mut q_foresight;

    if let Some(kn0) = n0.key_node() {
        kns = KnPile::new(kn0);
        q = VecDeque::new();
        q.push_back((0, n0));
        q_foresight = 0;
    }
    else {
        panic!();
    }

    loop {
        let q_size = q.len();

        let mut r = re.empty();
        let add_result = |r: &mut R, r1| {
            let r0 = std::mem::replace(r, re.empty());
            *r = re.reduce(r0, r1);
        };

        // Step one: expand q into q2
        let mut q2 = VecDeque::new();
        let mut q2_foresight = q_foresight + 1;
        while let Some((prev_idx, n)) = q.pop_front() {
            for n2 in ge.expand(&n) {
                let kn2 = n2.key_node();
                if let Some(kn2) = &kn2 {
                    if ge.end(kn2) {
                        let mut path = kns.materialize_cloned(prev_idx);
                        path.push(kn2.clone());
                        le.debug_end(&path);
                        add_result(&mut r, re.map_end(path));
                        continue;
                    }

                    let hn2 = kn2.hash_node();
                    let find_f = |idx, kn: &N::KN| {
                        if kn.hash_node() == hn2 {
                            Some(idx)
                        }
                        else {
                            None
                        }
                    };
                    if let Some(rep_idx) = kns.find(prev_idx, find_f) {
                        let mut path = kns.materialize_cloned(rep_idx);
                        path.pop().unwrap();
                        let mut cycle = kns.materialize_cloned(prev_idx);
                        let path2: Vec<_> = cycle.drain(0..path.len()).collect();
                        assert_eq!(path, path2);
                        le.debug_cycle(&path, &cycle, kn2);
                        add_result(&mut r, re.map_cycle(path, cycle, kn2.clone()));
                        continue;
                    }
                }

                q2.push_back((prev_idx, n2, kn2));
            }

            compact(&mut kns, &mut q, &mut q_foresight, &mut q2, &mut q2_foresight);
        }

        // Step two: fold q2 over into kns and q3
        let mut q3 = VecDeque::new();
        let mut q3_foresight = q2_foresight;
        while let Some((prev_idx, n, kn)) = q2.pop_front() {
            let mut prev_idx = prev_idx;
            if let Some(kn) = kn {
                prev_idx = kns.push(prev_idx, kn);
            }
            q3.push_back((prev_idx, n));

            compact(&mut kns, &mut q3, &mut q3_foresight, &mut q2, &mut q2_foresight);
        }

        eprintln!("Completed BFS step {} => {}", q_size, q3.len());

        // start over
        q = q3;

        let firstest = match q.front() {
            Some(&(idx, _)) => kns.materialize_cloned(idx),
            None => vec![],
        };
        le.on_recollect_firstest(firstest);
        if !le.on_recollect_results(r) {
            break;
        }
    }
}

fn compact<N: DfsNode>(kns: &mut KnPile<N::KN>, qa: &mut VecDeque<(usize, N)>, qa_foresight: &mut usize, qb: &mut VecDeque<(usize, N, Option<N::KN>)>, qb_foresight: &mut usize) {
    // whatever kns thinks plus (usize, usize) for space during recompaction
    loop {
        let kns_size = kns.len() * (kns.esize() + std::mem::size_of::<(usize, usize)>());
        let qa_size = qa.len() * std::mem::size_of::<(usize, N)>();
        let qb_size = qb.len() * std::mem::size_of::<(usize, N, Option<N::KN>)>();
        if kns_size + qa_size + qb_size <= (1 << 33) {
            return;
        }

        // TODO: compact
    }
}
