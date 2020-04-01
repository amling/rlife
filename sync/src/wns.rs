use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

struct WaitNotifyStateBe<S> {
    c: Condvar,
    m: Mutex<S>,
}

pub struct WaitNotifyState<S>(Arc<WaitNotifyStateBe<S>>);

impl<S> Clone for WaitNotifyState<S> {
    fn clone(&self) -> Self {
        return WaitNotifyState(self.0.clone());
    }
}

impl<S> WaitNotifyState<S> {
    pub fn new(s: S) -> Self {
        return WaitNotifyState(Arc::from(WaitNotifyStateBe {
            c: Condvar::new(),
            m: Mutex::new(s),
        }));
    }

    pub fn read<R>(&self, f: impl FnOnce(&S) -> R) -> R {
        let mg = self.0.m.lock().unwrap();
        return f(&mg);
    }

    pub fn write<R>(&self, f: impl FnOnce(&mut S) -> R) -> R {
        let mut mg = self.0.m.lock().unwrap();
        self.0.c.notify_all();
        return f(&mut mg);
    }

    pub fn wait<R>(&self, f: &mut impl FnMut(&mut S) -> (Option<R>, bool)) -> R {
        let mut mg = self.0.m.lock().unwrap();
        loop {
            let (r, n) = f(&mut mg);
            if n {
                self.0.c.notify_all();
            }
            if let Some(r) = r {
                return r;
            }
            mg = self.0.c.wait(mg).unwrap();
        }
    }
}
