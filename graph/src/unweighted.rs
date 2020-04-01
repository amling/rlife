use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

pub fn find_components<T: Hash + Eq + Clone>(links: &HashMap<T, HashSet<T>>) -> Vec<Vec<T>> {
    let mut already = HashSet::new();
    let mut acc = Vec::new();

    for t0 in links.keys() {
        if already.contains(t0) {
            continue;
        }

        let mut component = Vec::new();
        let mut q = vec![t0.clone()];

        loop {
            let t = match q.pop() {
                Some(t) => t,
                None => {
                    break;
                },
            };

            if !already.insert(t.clone()) {
                continue;
            }

            component.push(t.clone());
            if let Some(links2) = links.get(&t) {
                for t2 in links2 {
                    q.push(t2.clone());
                }
            }
        }

        acc.push(component);
    }

    acc
}
