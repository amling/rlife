use crate::bits;
use crate::dfs;

use bits::Bits;
use dfs::lifecycle::DfsLifecycle;
use dfs::res::DfsResVec;

pub struct GolLifecycle {
    pub threads: usize,
    pub recollect_ms: u64,
}

impl<B: Bits> DfsLifecycle<DfsResVec<(B, B)>> for GolLifecycle {
    fn threads(&self) -> usize {
        return self.threads;
    }

    fn recollect_ms(&self) -> u64 {
        return self.recollect_ms;
    }

    fn on_recollect(&self, r: DfsResVec<(B, B)>) -> bool {
        println!("Recollect...");

        // TODO: actual status
        println!("{:?}", r);

        return true;
    }
}
