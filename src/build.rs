use std::io::{BufRead, BufReader};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

use log::*;
use rocksdb::DB;

use crate::{force_get_json, LogUnwrap};
use crate::settings::{Config, Status};

pub fn handle(db: &DB, rebuild: bool, workdir: &Path) {
    let mut status = force_get_json::<Status>(db, "status");
    let config = force_get_json::<Config>(db, "config");
    if status.built && !rebuild {
        error!("already built");
        std::process::exit(1);
    }

    if status.in_progress.is_none() {
        error!("no current project");
        std::process::exit(1);
    }

    if status.mount.is_none() {
        error!("please init overlay layer first");
        std::process::exit(1);
    }

    let student = status.in_progress.as_ref().unwrap();

    let mount_point = status.mount.as_ref().unwrap();

    let data = workdir.join("student");
    let target = mount_point.join("data");

    info!("copy student {} to {}", data.display(), target.display());
    std::process::Command::new("sudo")
        .arg("-k")
        .arg("rsync")
        .arg("-r")
        .arg(format!("{}/", data.display()))
        .arg(target)
        .spawn()
        .exit_on_failure()
        .wait()
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("failed with {}", x)) })
        .exit_on_failure();

    let mut builder = std::process::Command::new("sudo");

    builder.arg("-k").arg("systemd-nspawn");
    if config.systemd_nspawn.no_network {
        builder.arg("--private-network");
    } else {
        builder.arg("--bind-ro=/etc/resolv.conf");
    }
    if config.systemd_nspawn.pid2 {
        builder.arg("--as-pid2");
    }
    if config.systemd_nspawn.no_new_privileges {
        builder.arg("--no-new-privileges");
    }
    if let Some(limit) = &config.systemd_nspawn.limit {
        if let Some(cpu) = limit.cpu_nums {
            builder.arg(format!("--cpu-affinity={}",
                                (0..cpu).map(|x| x.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(filesize) = limit.filesize_limit {
            builder.arg(format!("--rlimit=FSIZE={}", filesize));
        }
        if let Some(proc) = limit.process_limit {
            builder.arg(format!("--rlimit=NPROC={}", proc));
        }
        if let Some(nofile) = limit.nofile_limit {
            builder.arg(format!("--rlimit=NOFILE={}", nofile));
        }
        if let Some(sigpending) = limit.sigpending_limit {
            builder.arg(format!("--rlimit=SIGPENDING={}", sigpending));
        }
        if let Some(mem) = limit.mem_limit {
            builder.arg(format!("--rlimit=AS={}", mem));
        }
    }

    if let Some(dir) = &config.systemd_nspawn.work_path {
        builder.arg(format!("--chdir={}", dir.display()));
    }

    for i in &config.systemd_nspawn.env {
        builder.arg(format!("--setenv={}={}", i.name.as_str(), i.value.as_str()));
    }

    for i in &config.systemd_nspawn.capacity {
        builder.arg(format!("--capacity={}", i));
    }

    for i in &config.systemd_nspawn.capacity_drop {
        builder.arg(format!("--drop-capacity={}", i));
    }

    for i in &config.systemd_nspawn.extra_bind {
        let realpath = workdir.join(i.source.as_path()).canonicalize().exit_on_failure();
        builder.arg(format!("--bind={}:{}", realpath.display(), i.target.display()));
    }

    for i in &config.systemd_nspawn.extra_bind_ro {
        let realpath = workdir.join(i.source.as_path()).canonicalize().exit_on_failure();
        builder.arg(format!("--bind-ro={}:{}", realpath.display(), i.target.display()));
    }

    for i in &config.systemd_nspawn.extra_overlay {
        let realpath = workdir.join(i.source.as_path()).canonicalize().exit_on_failure();
        builder.arg(format!("--overlay={}:{}", realpath.display(), i.target.display()));
    }

    for i in &config.systemd_nspawn.extra_overlay_ro {
        let realpath = workdir.join(i.source.as_path()).canonicalize().exit_on_failure();
        builder.arg(format!("--overlay-ro={}:{}", realpath.display(), i.target.display()));
    }

    for i in &config.systemd_nspawn.syscall {
        if i.permit {
            builder.arg(format!("--system-call-filter={}", i.name));
        } else {
            builder.arg(format!("--system-call-filter=~{}", i.name));
        }
    }

    let shell = config.systemd_nspawn.shell
        .as_ref().map(|x|x.as_path()).unwrap_or("/bin/sh".as_ref());

    let mut child = builder.arg("--quiet")
        .arg("-D")
        .arg(mount_point)
        .arg(shell)
        .arg(format!("/data/{}", student.build_shell.display()))
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .exit_on_failure();

    let out = BufReader::new(child.stdout.take().unwrap());
    let err = BufReader::new(child.stderr.take().unwrap());
    let mut out_captured: Vec<u8> = Vec::new();
    let mut err_captured: Vec<u8> = Vec::new();
    let err_pointer = AtomicPtr::new(&mut err_captured as *mut Vec<u8>);
    let thread = thread::spawn(move || err.lines()
        .for_each(|line| unsafe {
            let l = line.unwrap_or(String::new());
            eprintln!("{}", l);
            if let Err(e) = writeln!(&mut *err_pointer.load(Relaxed), "{}", l) {
                error!("failed to record stderr: {}, error: {}", l, e);
            };
        }));

    out.lines().for_each(|line| {
        let l = line.unwrap_or(String::new());
        println!("{}", l);
        if let Err(e) = writeln!(&mut out_captured, "{}", l) {
            error!("failed to record stdout: {}, error: {}", l, e);
        };
    });
    if thread.join().is_err() {
        error!("failed to join io threads");
    };

    child.wait()
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("container exit with: {}", x)) })
        .exit_on_failure();

    status.built = true;
    status.stdout.replace(String::from_utf8_lossy(out_captured.as_slice()).to_string());
    status.stderr.replace(String::from_utf8_lossy(err_captured.as_slice()).to_string());
    db.put("status", serde_json::to_string(&status).exit_on_failure()).exit_on_failure();
}