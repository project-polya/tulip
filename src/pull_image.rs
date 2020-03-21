use std::path::Path;

use log::*;
use reqwest::Url;
use rocksdb::DB;
use serde::*;

use crate::{force_get, force_get_json, LogUnwrap};
use crate::settings::{Config, Status};

pub fn handle_local(db: &DB, workdir: &Path) {
    let mut status = force_get_json::<Status>(db, "status");
    if let Ok(meta) = std::fs::metadata(workdir.join("image/image.sfs")) {
        info!("target image detected with size: {} ", meta.len());
        status.image = true;
        db.put("status", serde_json::to_string(&status).exit_on_failure()).exit_on_failure();
    } else {
        error!("cannot access target path");
        std::process::exit(1);
    }
}

pub fn handle(force: bool, db: &DB, backend: &str, workdir: &Path) {
    let server = force_get(db, "server");

    let uuid = force_get(db, "uuid");

    let status = force_get_json::<Status>(db, "status");

    if status.image && !force {
        error!("image existed, exiting...");
        std::process::exit(1);
    }
    let request_url: Url = format!("{}/image.sfs", server).parse().exit_on_failure();
    let auth = format!("Authorization: Bearer {}", uuid);
    std::fs::create_dir_all(workdir.join("image")).exit_on_failure();
    match backend {
        "wget" => {
            std::process::Command::new("wget")
                .arg("-N")
                .arg("-P")
                .arg(workdir.join("image"))
                .arg(request_url.as_str())
                .arg("--header")
                .arg(auth)
                .arg("--show-progress")
                .spawn()
                .map_err(|x| x.to_string())
                .and_then(|mut x| x.wait().map_err(|x| x.to_string()))
                .and_then(|x| if x.success() { Ok(()) } else { Err(String::from("download failed")) })
                .exit_on_failure();
        }
        "aria2c" => {
            std::process::Command::new("aria2c")
                .arg(request_url.as_str())
                .arg("--auto-file-renaming=false")
                .arg("--optimize-concurrent-downloads")
                .arg("--dir")
                .arg(workdir.join("image"))
                .arg("-o")
                .arg("image.sfs")
                .arg("--header")
                .arg(auth)
                .spawn()
                .map_err(|x| x.to_string())
                .and_then(|mut x| x.wait().map_err(|x| x.to_string()))
                .and_then(|x| if x.success() { Ok(()) } else { Err(String::from("download failed")) })
                .exit_on_failure();
        }
        _ => unreachable!()
    }
    handle_local(db, workdir);
    refresh_config(server.as_str(), uuid.as_str(), db);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigResponse {
    config: Config
}

pub fn refresh_config(server: &str, uuid: &str, db: &DB) {
    let config = reqwest::blocking::Client::new()
        .get(format!("{}/config", server).as_str())
        .bearer_auth(uuid)
        .send()
        .and_then(|x| x.json::<ConfigResponse>())
        .exit_on_failure();
    if !config.config.notification.is_empty() {
        info!("server notification:\n{}", config.config.notification);
    }
    db.put(b"config", serde_json::to_vec(&config.config).exit_on_failure().as_mut_slice()).exit_on_failure();
}