pub trait DfsRes<N, R> {
    fn empty(&self) -> R;
    fn map_cycle(&self, _path: Vec<N>, _cycle: Vec<N>, _last: N) -> R {
        self.empty()
    }
    fn map_end(&self, path: Vec<N>) -> R;
    fn reduce(&self, r1: R, r2: R) -> R;
}

#[derive(Debug)]
pub struct DfsResVec<N> {
    pub cycles: Vec<(Vec<N>, Vec<N>, N)>,
    pub ends: Vec<Vec<N>>,
}

pub struct DfsResToVec();

impl<N> DfsRes<N, DfsResVec<N>> for DfsResToVec {
    fn empty(&self) -> DfsResVec<N> {
        DfsResVec {
            cycles: vec![],
            ends: vec![],
        }
    }

    fn map_cycle(&self, path: Vec<N>, cycle: Vec<N>, last: N) -> DfsResVec<N> {
        DfsResVec {
            cycles: vec![(path, cycle, last)],
            ends: vec![],
        }
    }

    fn map_end(&self, path: Vec<N>) -> DfsResVec<N> {
        DfsResVec {
            cycles: vec![],
            ends: vec![path],
        }
    }

    fn reduce(&self, mut r1: DfsResVec<N>, mut r2: DfsResVec<N>) -> DfsResVec<N> {
        r1.cycles.append(&mut r2.cycles);
        r1.ends.append(&mut r2.ends);
        r1
    }
}
