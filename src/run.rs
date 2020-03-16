use std::path::{Path, PathBuf};

use log::*;
use rocksdb::DB;

use crate::{force_get_json, LogUnwrap};
use crate::settings::{Config, Status};
use std::process::Stdio;
use std::io::{Write, BufReader, BufRead};
use std::sync::atomic::AtomicPtr;
use std::thread;
use std::sync::atomic::Ordering::Relaxed;

pub fn extra_bind(source: &Path, target: &Path, ro: bool) {
    info!("binding {} to {}", source.display(), target.display());
    let mut command = std::process::Command::new("sudo");
    command.arg("-k").arg("mount").arg("--bind");
    if ro { command.arg("-o").arg("ro"); }
    command.arg(source)
        .arg(target)
        .spawn()
        .exit_on_failure()
        .wait()
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("bind failed with {}", x)) })
        .exit_on_failure();
}

pub fn umount(target: &Path) {
    info!("umounting {}", target.display());
    let mut command = std::process::Command::new("sudo");
    if let Err(e) = command.arg("umount")
        .arg("-R")
        .arg(target)
        .spawn()
        .exit_on_failure()
        .wait()
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("umount failed with {}", x)) })
    {
        error!("{}", e);
    }
}

pub fn extra_overlay(source: &Path, target: &Path, workdir: &Path, ro: bool) -> Option<(PathBuf, PathBuf)> {
    info!("overlaying {} to {}", source.display(), target.display());
    let mut command = std::process::Command::new("sudo");
    command.arg("mount").arg("-t").arg("overlay").arg("overlay").arg("-o");
    let mut res = None;
    if !ro {
        let work = workdir.join("data").join(uuid::Uuid::new_v4().to_string());
        let upper = workdir.join("data").join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(work.as_path()).exit_on_failure();
        std::fs::create_dir_all(upper.as_path()).exit_on_failure();
        let work = work.canonicalize().exit_on_failure();
        let upper = upper.canonicalize().exit_on_failure();
        command.arg(format!("workdir={},upperdir={},lowerdir={}", work.display(), upper.display(), source.display()));
        res.replace((work, upper));
    } else {
        command.arg(format!("lowerdir={}", source.display()));
    }
    command.arg(target)
        .spawn()
        .exit_on_failure()
        .wait()
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("overlay failed with {}", x)) })
        .exit_on_failure();
    res
}

pub fn clear(target: &Path) {
    info!("clearing {}", target.display());
    if let Err(e) = std::process::Command::new("sudo")
        .arg("-k")
        .arg("rm")
        .arg("-rf")
        .arg(target)
        .spawn()
        .map_err(|x| x.to_string())
        .and_then(|mut x| {
            x.wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("clearing failed with {}", x)) })
        })
    {
        error!("{}", e);
    }
}

pub fn mkdir(target: &Path) {
    info!("making dir {}", target.display());
    std::process::Command::new("sudo")
        .arg("-k")
        .arg("mkdir")
        .arg("-p")
        .arg(target)
        .spawn()
        .map_err(|x| x.to_string())
        .and_then(|mut x| {
            x.wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("mkdir failed with {}", x)) })
        }).exit_on_failure();
}

pub fn run(db: &DB, workdir: &Path, without_build: bool) {
    let config = force_get_json::<Config>(db, "config");

    let status = force_get_json::<Status>(db, "status");

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

    let mount_point = status.mount.unwrap();

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
        .map(|x|x.name.as_str())
        .collect::<Vec<_>>().join(",");
    let permit: String = firejail.syscall.iter().filter(|x| x.permit)
        .map(|x|x.name.as_str())
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
        stdin.replace(std::fs::read_to_string(path).exit_on_failure());
    }

    let mut mounting = Vec::new();
    let mut garbage = Vec::new();

    for i in &config.systemd_nspawn.extra_bind {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        extra_bind(workdir.join(i.source.as_path()).as_path(),
                   target.as_path(), false);
        mounting.push(target.clone());
    }

    for i in &config.systemd_nspawn.extra_bind_ro {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        extra_bind(workdir.join(i.source.as_path()).as_path(),
                   target.as_path(), true);
        mounting.push(target.clone());
    }



    for i in &config.systemd_nspawn.extra_overlay {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        for i in extra_overlay(workdir.join(i.source.as_path()).as_path(),
                               target.as_path(), workdir, false) {
            garbage.push(i.0);
            garbage.push(i.1);
        };
        mounting.push(target.clone());
    }

    for i in &config.systemd_nspawn.extra_overlay_ro {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        for i in extra_overlay(workdir.join(i.source.as_path()).as_path(),
                               target.as_path(), workdir, true) {
            garbage.push(i.0);
            garbage.push(i.1);
        };
        mounting.push(target.clone());
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


    let mut child = command.arg(shell)
        .arg(format!("/data/{}", student.run_shell.display()))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            error!("{}", e);
            clear_these(&mounting, &garbage);
            std::process::exit(1);
        });
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
        .map_err(|x|x.to_string())
        .and_then(|x| if x.success() {Ok(())} else {Err(format!("failed with {}", x))})
        .unwrap_or_else(|e| {
            error!("{}", e);
            clear_these(&mounting, &garbage);
            std::process::exit(1);
        });

    clear_these(&mounting, &garbage);
}

fn clear_these(mounting: &Vec<PathBuf>, garbage: &Vec<PathBuf>) {
    for i in mounting {
        umount(i.as_path());
    }

    for i in garbage {
        clear(i.as_path());
    }
}