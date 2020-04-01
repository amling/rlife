use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

use crate::zmodule;

use zmodule::ZModule;

pub fn egcd<R: ZModule>(mut a: isize, mut b: isize, mut ra: R, mut rb: R) -> (isize, R, R) {
    egcd_mut(&mut a, &mut b, &mut ra, &mut rb);
    (b, ra, rb)
}

pub fn egcd_mut<R: ZModule>(a: &mut isize, b: &mut isize, ra: &mut R, rb: &mut R) {
    if *a < 0 {
        *a *= -1;
        ra.mul(-1);
    }

    if *b < 0 {
        *b *= -1;
        rb.mul(-1);
    }

    while *a > 0 {
        let q = *b / *a;

        *b -= q * *a;
        rb.addmul(-q, &ra);

        std::mem::swap(a, b);
        std::mem::swap(ra, rb);
    }
}

// Finds generators for the space of cycle weights reachable from the given node.
pub fn find_cycle_weight_generators<N: Hash + Clone + Eq, R: ZModule + Clone>(links: &HashMap<N, HashSet<(N, R)>>, n: N) -> Vec<R> {
    let connected_weights = find_connected_weights(links, n);

    let mut acc = Vec::new();
    for (n1, links2) in links {
        for (n2, r) in links2 {
            if let Some(r1) = connected_weights.get(n1) {
                if let Some(r2) = connected_weights.get(n2) {
                    let mut label = r1.clone();
                    label.addmul(1, r);
                    label.addmul(-1, r2);
                    acc.push(label);
                }
            }
        }
    }
    acc
}

// Finds the set of connected nodes and the weight of some arbitrary path to them
pub fn find_connected_weights<N: Hash + Clone + Eq, R: ZModule + Clone>(links: &HashMap<N, HashSet<(N, R)>>, n: N) -> HashMap<N, R> {
    let mut acc = HashMap::new();
    find_connected_weights_aux(links, &mut acc, n, R::zero());
    acc
}

fn find_connected_weights_aux<N: Hash + Clone + Eq, R: ZModule + Clone>(links: &HashMap<N, HashSet<(N, R)>>, acc: &mut HashMap<N, R>, n: N, r: R) {
    if let Some(_) = acc.get(&n) {
        return;
    }

    acc.insert(n.clone(), r.clone());

    for (n2, rd) in links.get(&n).unwrap() {
        let mut r2 = r.clone();
        r2.addmul(1, rd);
        find_connected_weights_aux(links, acc, n2.clone(), r2);
    }
}
