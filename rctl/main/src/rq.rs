use ars_rctl_core::RctlLog;
use ars_sync::wns::WaitNotifyState;
use std::collections::VecDeque;

struct Hub<R> {
    outputs: VecDeque<String>,
    ret: Option<R>,
}

enum HubRead<R> {
    Output(String),
    Ret(R)
}

impl<R> Hub<R> {
    fn new() -> Hub<R> {
        Hub {
            outputs: VecDeque::new(),
            ret: None,
        }
    }

    fn output(&mut self, line: String) {
        self.outputs.push_back(line);
    }

    fn ret(&mut self, ret: R) {
        self.ret.replace(ret);
    }

    fn read(&mut self) -> Option<HubRead<R>> {
        if let Some(line) = self.outputs.pop_front() {
            return Some(HubRead::Output(line));
        }
        if let Some(r) = self.ret.take() {
            return Some(HubRead::Ret(r));
        }
        None
    }
}

pub struct RctlRunQueue<I, R> {
    q: WaitNotifyState<VecDeque<(I, WaitNotifyState<Hub<R>>)>>,
}

impl<I, R> RctlRunQueue<I, R> {
    pub fn new() -> RctlRunQueue<I, R> {
        RctlRunQueue {
            q: WaitNotifyState::new(VecDeque::new()),
        }
    }

    pub fn run(&self, req: I, mut log: RctlLog) -> R {
        let hub = WaitNotifyState::new(Hub::new());

        self.q.write(|q| q.push_back((req, hub.clone())));

        loop {
            match hub.wait(&mut |hub| (hub.read(), false)) {
                HubRead::Output(line) => log.log(line),
                HubRead::Ret(ret) => {
                    return ret;
                }
            }
        }
    }

    pub fn service(&self, f: &mut impl FnMut(I, RctlLog) -> R) {
        if let Some(pair) = self.q.write(|q| q.pop_front()) {
            self.service_one(f, pair);
        }
    }

    pub fn service_blocking(&self, f: &mut impl FnMut(I, RctlLog) -> R) {
        let pair = self.q.wait(&mut |q| (q.pop_front(), false));
        self.service_one(f, pair);
    }

    fn service_one(&self, f: &mut impl FnMut(I, RctlLog) -> R, (req, hub): (I, WaitNotifyState<Hub<R>>)) {
        let log = {
            let hub = hub.clone();
            RctlLog(Box::new(move |line| {
                hub.write(|hub| {
                    hub.output(line);
                });
            }))
        };

        let res = f(req, log);

        hub.write(|hub| {
            hub.ret(res);
        });
    }
}
