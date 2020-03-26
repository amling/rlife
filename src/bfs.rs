use std::collections::HashMap;
use std::hash::Hash;

use crate::dfs;

use dfs::graph::DfsGraph;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsRes;

pub fn sbfs<N: Clone + Hash + Eq, R, GE: DfsGraph<N>, RE: DfsRes<N, R>, LE: DfsLifecycle<N, R>>(ge: &GE, re: &RE, le: &LE) {
    let mut q: Vec<(Vec<N>, HashMap<N, usize>, N)> = Vec::new();

    let n0 = ge.start();

    q.push((vec![n0.clone()], vec![(n0.clone(), 0)].into_iter().collect(), n0));

    while q.len() > 0 {
        let mut q2 = Vec::new();
        let mut r = re.empty();
        for (path, already, n) in q {
            for n2 in ge.expand(&n) {
                let mut path2 = path.clone();
                let idx = path.len();
                path2.push(n2.clone());
                let mut already2 = already.clone();
                already2.insert(n2.clone(), idx);

                if ge.end(&n2) {
                    le.debug_end(&path2);
                    r = re.reduce(r, re.map_end(path2));
                    continue;
                }

                if let Some(&idx) = already.get(&n2) {
                    let mut path = path.clone();
                    let cycle = path.drain(idx..).collect();
                    le.debug_cycle(&path, &cycle);
                    r = re.reduce(r, re.map_cycle(path, cycle));
                    continue;
                }

                q2.push((path2, already2, n2));
            }
        }
        let firstest = match q2.first() {
            Some(&(ref path, _, _)) => path.clone(),
            None => vec![],
        };
        le.on_recollect(firstest, r);
        q = q2;
    }
}
