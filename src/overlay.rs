use std::path::*;

use log::*;
use rocksdb::DB;

use crate::{force_get_json, LogUnwrap};
use crate::settings::{Config, Status};

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

pub fn handle(db: &DB, workdir: &Path, nutshell: &Path, print_result: bool, shell: bool, mount_point: &Path, tmp_size: Option<usize>, force: bool) {
    let config = force_get_json::<Config>(db, "config");
    let mut status = force_get_json::<Status>(db, "status");
    if !status.image {
        error!("please pull down a base image first");
        std::process::exit(1);
    }
    if status.in_progress.is_none() {
        error!("please fetch a student project first");
        std::process::exit(1);
    }
    if status.mount.is_some() && !force {
        error!("please umount the current overlay system first");
        std::process::exit(1);
    }

    info!("initializing the data dir");

    std::fs::create_dir_all(workdir.join("data")).exit_on_failure();

    info!("starting nutshell process");

    let mut command = std::process::Command::new("sudo");
    command
        .arg("-k")
        .arg("-E")
        .arg(nutshell)
        .arg("init-overlay")
        .arg("-m")
        .arg(mount_point.canonicalize().exit_on_failure())
        .arg("-d")
        .arg(workdir.join("data").canonicalize().exit_on_failure())
        .arg("-b")
        .arg(workdir.join("image/root.x86_64").canonicalize().exit_on_failure());
    if let Some(size) = tmp_size {
        command.arg("-t").arg(format!("{}m", size));
    }
    if shell {
        command.arg("-s");
    }
    if print_result {
        command.arg("-p");
    }
    let res = command.spawn().exit_on_failure().wait().exit_on_failure();
    if !res.success() {
        error!("failed with {}", res);
        std::process::exit(1);
    }
    status.mount.replace(mount_point.canonicalize().exit_on_failure());
    db.put("status", serde_json::to_string(&status).exit_on_failure()).exit_on_failure();

    for i in &config.extra_bind {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        extra_bind(workdir.join("image").join(i.source.as_path()).as_path(),
                   target.as_path(), false);
    }

    for i in &config.extra_bind_ro {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        extra_bind(workdir.join("image").join(i.source.as_path()).as_path(),
                   target.as_path(), true);
    }

    for i in &config.extra_overlay {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        extra_overlay(workdir.join("image").join(i.source.as_path()).as_path(),
                      target.as_path(), workdir, false);
    }

    for i in &config.extra_overlay_ro {
        let target = mount_point.join(i.target.as_path());
        mkdir(target.as_path());
        extra_overlay(workdir.join("image").join(i.source.as_path()).as_path(),
                      target.as_path(), workdir, true);
    }
}

pub fn handle_destroy(db: &DB, workdir: &Path) {
    let mut status = force_get_json::<Status>(db, "status");
    if let Some(mount) = &status.mount {
        info!("trying to umount {}", mount.display());
        let umount = std::process::Command::new("sudo").arg("-k")
            .arg("umount")
            .arg("-R")
            .arg(mount)
            .spawn()
            .and_then(|mut x| x.wait());
        match umount {
            Ok(e) => info!("umount exit with {}", e),
            Err(e) => error!("umount failed with {}", e)
        }
    }
    let deleting_path = workdir.join("data");
    warn!("deleting {}", deleting_path.display());
    let deleting = std::process::Command::new("sudo").arg("-k")
        .arg("rm")
        .arg("-rf")
        .arg(deleting_path)
        .spawn()
        .and_then(|mut x| x.wait());
    match deleting {
        Ok(e) => info!("deleting exit with {}", e),
        Err(e) => error!("deleting failed with {}", e)
    }
    status.built = false;
    status.mount = None;
    db.put("status", serde_json::to_vec(&status).exit_on_failure()).exit_on_failure();
}
