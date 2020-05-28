use ars_ds::err::StringError;
use ars_rctl_core::RctlEp;
use ars_rctl_core::RctlLog;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

pub mod rq;

pub fn spawn(ep: Arc<impl RctlEp + 'static>) {
    std::thread::spawn(|| {
        if let Err(e) = event_loop(ep) {
            eprintln!("rctl event loop crashed: {:?}", e);
        }
    });
}

enum Impossible {
}

// Errors from here crash the event loop
fn event_loop(ep: Arc<impl RctlEp + 'static>) -> Result<Impossible, StringError> {
    let dir = format!("{}/.rctl", std::env::var("HOME")?);
    std::fs::create_dir_all(&dir)?;

    let sock = format!("{}/{}.sock", dir, std::process::id());
    let sock = UnixListener::bind(sock)?;

    for sock in sock.incoming() {
        let ep = ep.clone();
        let sock = sock.map_err(StringError::from);
        if let Err(e) = handle1(ep, sock) {
            eprintln!("rctl event loop error from handle1: {:?}", e);
        }
    }

    Err(StringError::new("Listener closed?"))
}

// Errors from here crash the handle call
fn handle1(ep: Arc<impl RctlEp + 'static>, sock: Result<UnixStream, StringError>) -> Result<(), StringError> {
    let sock = sock?;
    std::thread::spawn(move || {
        if let Err(e) = handle2(ep, sock) {
            eprintln!("rctl handler for crashed: {:?}", e);
        }
    });
    Ok(())
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub enum RctlRequest {
    Metadata,
    Invoke(String, Vec<Value>),
}

#[derive(Deserialize)]
#[derive(Serialize)]
pub enum RctlResponse {
    Error(String),
    Output(String),
    Ret(Value),
}

// Errors from here crash the handler thread
fn handle2(ep: Arc<impl RctlEp>, sock: UnixStream) -> Result<(), StringError> {
    let sockr = sock.try_clone()?;
    let sockw = sock.try_clone()?;
    let mut bw = BufWriter::new(sockw);

    let mut write_res = |res| -> Result<(), StringError> {
        let s = serde_json::to_string(&res)?;
        writeln!(bw, "{}", s)?;
        Ok(())
    };
    let mut log_err = None;

    let log = RctlLog(Box::new(|line| {
        if let Err(e) = write_res(RctlResponse::Output(line)) {
            log_err.get_or_insert(e);
        }
    }));

    let res = match handle3(ep, sockr, log) {
        Ok(None) => {
            return Ok(());
        }
        Ok(Some(res)) => RctlResponse::Ret(res),
        Err(e) => RctlResponse::Error(e.msg),
    };

    if let Some(e) = log_err {
        return Err(e);
    }

    write_res(res)?;

    Ok(())
}

// Errors from here we'll try to write to client
fn handle3<E: RctlEp>(ep: Arc<E>, mut sockr: UnixStream, log: RctlLog) -> Result<Option<Value>, StringError> {
    let mut s = String::new();
    sockr.read_to_string(&mut s)?;
    if s.len() == 0 {
        return Ok(None);
    }
    let req = serde_json::from_str(&s)?;

    let res = match req {
        RctlRequest::Metadata => serde_json::to_value(E::metadata())?,
        RctlRequest::Invoke(name, args) => ep.invoke(log, name, &args)?,
    };

    Ok(Some(res))
}
