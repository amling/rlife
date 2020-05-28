use ars_ds::err::StringError;
use ars_rctl_core::RctlMethodMetadata;
use ars_rctl_main::RctlRequest;
use ars_rctl_main::RctlResponse;
use serde_json::Value;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::ErrorKind;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use sysinfo::System;
use sysinfo::SystemExt;

fn main() {
    let mut system = System::new();

    system.refresh_all();

    let mut procs: Vec<_> = system.get_process_list().into_iter().map(|(_, proc)| proc).collect();
    procs.retain(|proc| ping(proc.pid).unwrap());

    let args: Vec<_> = std::env::args().skip(1).collect();

    let mut args: &[_] = &args;
    loop {
        // TODO: make this less fucking horrible when those rust fuckers get off their asses and
        // release subslice patterns
        if args.len() >= 1 && args[0] == "--here" {
            let pwd = std::env::current_dir().unwrap();
            let pwd = pwd.to_str().unwrap();
            procs.retain(|proc| proc.cwd == pwd);
            args = &args[1..];
            continue;
        }
        if args.len() >= 2 && args[0] == "--cwd" {
            let dir = &args[1];
            procs.retain(|proc| &proc.cwd == dir);
            args = &args[2..];
            continue;
        }
        if args.len() >= 2 && args[0] == "--pid" {
            let pid: i32 = args[1].parse().unwrap();
            procs.retain(|proc| proc.pid == pid);
            args = &args[2..];
            continue;
        }
        if args.len() >= 2 && args[0] == "--name" {
            let name = &args[1];
            procs.retain(|proc| &proc.name == name);
            args = &args[2..];
            continue;
        }
        break;
    }

    let show_procs = || {
        println!("Selected {} processes:", procs.len());
        for proc in procs.iter() {
            println!("{}: {} ({})", proc.pid, proc.name, proc.cwd);
        }
    };

    match args.split_first() {
        None => show_procs(),
        Some((cmd, args)) => {
            if procs.len() != 1 {
                show_procs();
                std::process::exit(1);
            }
            let pid = procs[0].pid;
            match cmd as &str {
                "metadata" => {
                    assert!(args.len() == 0, "No arguments expected for metadata");
                    let res = call(pid, RctlRequest::Metadata).unwrap();
                    let metadata: Vec<(String, RctlMethodMetadata)> = serde_json::from_value(res).unwrap();
                    for (name, metadata) in metadata {
                        let args = metadata.args.iter().map(|arg| {
                            format!("{}: {}", arg.name, arg.ty.s)
                        }).collect::<Vec<_>>().join(", ");
                        println!("{}({}) -> {}", name, args, metadata.ret.s);
                    }
                },
                "invoke" => {
                    assert!(args.len() >= 1, "Arguments expected for invoke");
                    let method = args[0].clone();
                    let args = args[1..].iter().map(|s| serde_json::from_str::<Value>(s).unwrap()).collect();
                    let res = call(pid, RctlRequest::Invoke(method, args)).unwrap();
                    println!("{}", serde_json::to_string(&res).unwrap());
                },
                _ => panic!("Unknown command: {}", cmd),
            };
        }
    }
}

fn ping(pid: i32) -> Result<bool, StringError> {
    let dir = format!("{}/.rctl", std::env::var("HOME")?);
    let sock = format!("{}/{}.sock", dir, pid);

    match UnixStream::connect(sock) {
        Ok(_) => Ok(true),
        Err(e) => match e.kind() {
            // no socket, not an rctl process
            ErrorKind::NotFound => Ok(false),

            // no one was listening, also not an rctl process (could have been stranded by previous
            // rctl process of same PID)
            ErrorKind::ConnectionRefused => Ok(false),

            // unknown, bail out
            _ => Err(StringError::from(e)),
        },
    }
}

fn call(pid: i32, req: RctlRequest) -> Result<Value, StringError> {
    let dir = format!("{}/.rctl", std::env::var("HOME")?);
    let sock = format!("{}/{}.sock", dir, pid);
    let sock = UnixStream::connect(sock)?;

    let sockr = sock.try_clone()?;
    let sockw = sock.try_clone()?;

    let bw = BufWriter::new(sockw);
    serde_json::to_writer(bw, &req)?;
    sock.shutdown(Shutdown::Write)?;

    let br = BufReader::new(sockr);
    for line in br.lines() {
        let line = line?;
        let res: RctlResponse = serde_json::from_str(&line)?;

        match res {
            RctlResponse::Error(msg) => {
                return Err(StringError::new(msg));
            }
            RctlResponse::Output(line) => {
                eprintln!("Output: {}", line);
            }
            RctlResponse::Ret(res) => {
                return Ok(res);
            }
        }
    }

    Err(StringError::new("Got no return?"))
}
