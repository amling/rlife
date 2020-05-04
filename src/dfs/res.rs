#[derive(Debug)]
pub struct DfsRes<N> {
    pub cycles: Vec<(Vec<N>, Vec<N>, N)>,
    pub ends: Vec<(Vec<N>, &'static str)>,
}

impl<N> DfsRes<N> {
    pub fn new() -> Self {
        DfsRes {
            cycles: Vec::new(),
            ends: Vec::new(),
        }
    }

    pub fn add_cycle(&mut self, path: Vec<N>, cycle: Vec<N>, last: N) {
        self.cycles.push((path, cycle, last));
    }

    pub fn add_end(&mut self, path: Vec<N>, label: &'static str) {
        self.ends.push((path, label));
    }

    pub fn append(&mut self, other: &mut DfsRes<N>) {
        self.cycles.append(&mut other.cycles);
        self.ends.append(&mut other.ends);
    }
}
