use ars_rctl_core::RctlLog;
use ars_sync::wns::WaitNotifyState;
use std::collections::VecDeque;

struct RctlDeferredBackend<R> {
    outputs: VecDeque<String>,
    ret: Option<R>,
}

pub struct RctlDeferredRead<R>(WaitNotifyState<RctlDeferredBackend<R>>);

impl<R> RctlDeferredRead<R> {
    pub fn wait(self, mut log: RctlLog) -> R {
        self.0.wait(&mut |be| {
            while let Some(line) = be.outputs.pop_front() {
                log.log(line);
            }
            if let Some(r) = be.ret.take() {
                return (Some(r), false);
            }
            return (None, false);
        })
    }
}

pub struct RctlDeferredWrite<R>(WaitNotifyState<RctlDeferredBackend<R>>);

impl<R> RctlDeferredWrite<R> {
    pub fn output(&mut self, line: impl Into<String>) {
        self.0.write(|be| be.outputs.push_back(line.into()));
    }

    pub fn ret(self, ret: R) {
        self.0.write(|be| be.ret.replace(ret));
    }
}

pub fn deferred<R>() -> (RctlDeferredRead<R>, RctlDeferredWrite<R>) {
    let be = RctlDeferredBackend {
        outputs: VecDeque::new(),
        ret: None,
    };
    let be = WaitNotifyState::new(be);

    (RctlDeferredRead(be.clone()), RctlDeferredWrite(be))
}

pub struct RctlRunQueue<T>(WaitNotifyState<VecDeque<T>>);

impl<T> RctlRunQueue<T> {
    pub fn new() -> RctlRunQueue<T> {
        RctlRunQueue(WaitNotifyState::new(VecDeque::new()))
    }

    pub fn push(&self, t: T) {
        self.0.write(|q| q.push_back(t));
    }

    pub fn service(&self, f: &mut impl FnMut(T)) {
        if let Some(t) = self.0.write(|q| q.pop_front()) {
            f(t);
        }
    }

    pub fn service_blocking(&self, f: &mut impl FnMut(T)) {
        let t = self.0.wait(&mut |q| (q.pop_front(), false));
        f(t);
    }
}
