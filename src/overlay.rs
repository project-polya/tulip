use std::path::Path;

use log::*;
use rocksdb::DB;

use crate::{force_get_json, LogUnwrap};
use crate::settings::Status;

pub fn handle(db: &DB, workdir: &Path, nutshell: &Path, print_result: bool, shell: bool, mount_point: &Path, tmp_size: Option<usize>, force: bool) {
    let mut status = force_get_json::<Status>(db, "status");
    if !status.image {
        error!("please pull down a base image first");
        std::process::exit(1);
    }
    if!status.in_progress.is_none() {
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
    command.arg("-E")
        .arg(nutshell)
        .arg("init-overlay")
        .arg("-m")
        .arg(mount_point.canonicalize().exit_on_failure())
        .arg("-d")
        .arg(workdir.join("data").canonicalize().exit_on_failure())
        .arg("-b")
        .arg(workdir.join("root.x86_64").canonicalize().exit_on_failure());
    if let Some(size) = tmp_size {
        command.arg("-t").arg(size.to_string());
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
}

pub fn handle_destroy(db: &DB, workdir: &Path){
    let mut status = force_get_json::<Status>(db, "status");
    if let Some(mount) = &status.mount {
        info!("trying to umount {}", mount.display());
        let umount = std::process::Command::new("sudo")
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
    let deleting = std::process::Command::new("sudo")
        .arg("rm")
        .arg("-rf")
        .arg(deleting_path)
        .spawn()
        .and_then(|mut x|x.wait());
    match deleting {
        Ok(e ) => info!("deleting exit with {}", e),
        Err(e) => error!("deleting failed with {}", e)
    }
    status.built = false;
    status.mount = None;
    db.put("status", serde_json::to_vec(&status).exit_on_failure()).exit_on_failure();
}
