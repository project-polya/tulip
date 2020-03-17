use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

use log::*;
use rocksdb::DB;

use crate::{force_get_json, LogUnwrap};
use crate::settings::{Config, Status};

pub fn run(db: &DB, workdir: &Path, without_build: bool) {
    let config = force_get_json::<Config>(db, "config");

    let mut status = force_get_json::<Status>(db, "status");

    if status.in_progress.is_none() {
        error!("no current project");
        std::process::exit(1);
    }

    if status.mount.is_none() {
        error!("please init overlay layer first");
        std::process::exit(1);
    }

    if !status.built && !without_build {
        error!("please build the project first");
        std::process::exit(1);
    }

    let student = status.in_progress.as_ref().unwrap();

    let mount_point = status.mount.as_ref().unwrap();

    let firejail = &config.firejail;

    let mut command = std::process::Command::new("firejail");

    command.arg("--quiet")
        .arg("--deterministic-exit-code")
        .arg(format!("--chroot={}", mount_point.display()));

    if firejail.nodefault {
        command.arg("--noprofile");
    }

    let shell = config.firejail.shell
        .as_ref().map(|x| x.as_str()).unwrap_or("/bin/sh".as_ref());

    command.arg(format!("--shell={}", shell));


    if let Some(limit) = firejail.limit.as_ref() {
        if let Some(cpu) = limit.cpu_nums {
            command.arg(format!("--cpu={}",
                                (0..cpu).map(|x| x.to_string()).collect::<Vec<_>>().join(",")));
        }
        if let Some(filesize) = limit.filesize_limit {
            command.arg(format!("--rlimit-fsize={}", filesize));
        }
        if let Some(proc) = limit.process_limit {
            command.arg(format!("--rlimit-nproc={}", proc));
        }
        if let Some(nofile) = limit.nofile_limit {
            command.arg(format!("--rlimit-nofile={}", nofile));
        }
        if let Some(sigpending) = limit.sigpending_limit {
            command.arg(format!("--rlimit-sigpending={}", sigpending));
        }
        if let Some(mem) = limit.mem_limit {
            command.arg(format!("--rlimit-as={}", mem));
        }
    }

    if let Some(timeout) = firejail.timeout.as_ref() {
        command.arg(format!("--timeout={}:{}:{}", timeout.hour, timeout.minute, timeout.second));
    }

    if let Some(profile) = firejail.with_profile.as_ref() {
        command.arg(format!("--profile={}", workdir.join(profile).display()));
    }

    if let Some(mac) = firejail.mac.as_ref() {
        command.arg(format!("--mac={}", mac));
    }

    if let Some(dns) = firejail.dns.as_ref() {
        for i in dns {
            command.arg(format!("--dns={}", i));
        }
    }


    if let Some(nice) = firejail.nice.as_ref() {
        command.arg(format!("--nice={}", nice));
    }

    let block: String = firejail.syscall.iter().filter(|x| !x.permit)
        .map(|x| x.name.as_str())
        .collect::<Vec<_>>().join(",");
    let permit: String = firejail.syscall.iter().filter(|x| x.permit)
        .map(|x| x.name.as_str())
        .collect::<Vec<_>>().join(",");

    if !block.is_empty() {
        command.arg(format!("--seccomp.block={}", block));
    }

    if !permit.is_empty() {
        command.arg(format!("--seccomp={}", permit));
    }

    let cap: String = firejail.capacity.join(",");

    let cap_drop: String = firejail.capacity_drop.join(",");

    if !cap.is_empty() {
        command.arg(format!("--caps.keep={}", cap));
    }

    if !cap_drop.is_empty() {
        command.arg(format!("--caps.drop={}", permit));
    }

    let mut stdin = None;
    if let Some(path) = &config.stdin {
        stdin.replace(std::fs::read_to_string(workdir.join("image").join(path)).exit_on_failure());
    }


    if firejail.function.no3d {
        command.arg("--no3d");
    }

    if firejail.function.noautopulse {
        command.arg("--noautopulse");
    }

    if firejail.function.nodbus {
        command.arg("--nodbus");
    }

    if firejail.function.nodvd {
        command.arg("--nodvd");
    }

    if firejail.function.nogroups {
        command.arg("--nogroups");
    }

    if firejail.function.nonewprivs {
        command.arg("--nonewprivs");
    }

    if firejail.function.nou2f {
        command.arg("--nou2f");
    }

    if firejail.function.novideo {
        command.arg("--novideo");
    }

    if firejail.function.nonet {
        command.arg("--net=none");
    }

    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    if firejail.has_x {
        info!("adjust xhost");
        std::process::Command::new("xhost").arg("+")
            .spawn()
            .exit_on_failure()
            .wait()
            .exit_on_failure();
    }

    for i in &config.extra_bind {
        command.arg(format!("--whitelist={}", mount_point.join(i.target.as_path()).display()));
    }

    for i in &config.extra_overlay {
        command.arg(format!("--whitelist=/{}", mount_point.join(i.target.as_path()).display()));
    }

    command.arg(format!("--whitelist={}", mount_point.join("data").display()));

    let mut child = command.arg(shell)
        .arg(format!("/data/{}", student.run_shell.display()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .exit_on_failure();

    info!("running start");

    if let Some(content) = stdin {
        if let Err(e) = child.stdin.take().unwrap().write_all(content.as_bytes()) {
            error!("failed to write stdin: {}", e);
        }
    }


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
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("failed with {}", x)) })
        .exit_on_failure();

    if firejail.has_x {
        info!("ban connections to xhost");
        std::process::Command::new("xhost").arg("-")
            .spawn()
            .exit_on_failure()
            .wait()
            .exit_on_failure();
    }

    status.stderr.replace(String::from_utf8_lossy(err_captured.as_slice()).to_string());
    status.stdout.replace(String::from_utf8_lossy(out_captured.as_slice()).to_string());
    db.put("status", serde_json::to_vec(&status).exit_on_failure()).exit_on_failure();
}
