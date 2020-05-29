#[cfg(test)]
use ars_rctl_core::RctlEp;

use ars_rctl_core::RctlLog;
use ars_rctl_derive::rctl_ep;
use ars_rctl_main::rq::RctlRunQueue;
use std::fmt::Debug;
use std::sync::Arc;

struct Structo<T> {
    t: T,
    n_rq: RctlRunQueue<Box<dyn FnOnce(&mut usize) + Send>>,
}

#[rctl_ep]
impl<T: Debug + Send + Sync> Structo<Box<T>> {
    fn cooler(&self, s: String) -> String {
        format!("[{:?}] {}!", self.t, s)
    }

    fn loggo(&self, mut log: RctlLog) {
        log.log("Hello, world!!!");
    }

    fn foo(&self) {
    }

    fn inc(&self, log: RctlLog) {
        let (r, mut w) = ars_rctl_main::rq::deferred();
        self.n_rq.push(Box::new(|n| {
            *n += 1;
            w.output(format!("n incremented to {}", *n));
            w.ret(());
        }));
        r.wait(log)
    }
}

#[test]
fn metadata_runs() {
    <Structo<Box<usize>>>::metadata();
}

#[test]
fn invoke_runs() {
    let s = Structo {
        t: Box::new("abc"),
        n_rq: RctlRunQueue::new(),
    };

    assert_eq!("[\"abc\"] def!", serde_json::from_value::<String>(s.invoke(RctlLog::ignore(), "cooler", &vec![serde_json::to_value("def").unwrap()]).unwrap()).unwrap());
}

fn main() {
    let s = Structo {
        t: Box::new("abc"),
        n_rq: RctlRunQueue::new(),
    };
    let s = Arc::new(s);

    ars_rctl_main::spawn(s.clone());

    let mut n = 0;
    loop {
        s.n_rq.service_blocking(&mut |f| {
            f(&mut n)
        });
    }
}
