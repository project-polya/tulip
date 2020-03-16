use std::path::Path;

use log::*;
use reqwest::Url;
use rocksdb::DB;
use serde::*;

use crate::{force_get, force_get_json, LogUnwrap};
use crate::settings::{Config, Status};

pub fn handle_local(db: &DB, workdir: &Path) {
    let mut status = force_get_json::<Status>(db, "status");
    if let Ok(meta) = std::fs::metadata(workdir.join("root.x86_64")) {
        if meta.is_dir() {
            info!("target image detected");
            status.image = true;
            db.put("status", serde_json::to_string(&status).exit_on_failure()).exit_on_failure();
        } else {
            error!("target path is not a directory");
            std::process::exit(1);
        }
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
    let request_url: Url = format!("{}/image.tar", server).parse().exit_on_failure();
    let auth = format!("Authorization: Bearer {}", uuid);
    match backend {
        "wget" => {
            std::process::Command::new("wget")
                .arg("-N")
                .arg("-P")
                .arg("/tmp")
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
                .arg("--optimize-concurrent-downloads")
                .arg("--dir")
                .arg("/tmp")
                .arg("-o")
                .arg("image.tar")
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
    let untar_path = std::fs::canonicalize(workdir).exit_on_failure();
    info!("untar image to {}", untar_path.display());
    std::process::Command::new("sudo")
        .arg("-k")
        .arg("tar")
        .arg("-C")
        .arg(untar_path)
        .arg("-xf")
        .arg("/tmp/image.tar")
        .spawn()
        .map_err(|x| x.to_string())
        .and_then(|mut x| x.wait().map_err(|x| x.to_string()))
        .and_then(|x| if x.success() { Ok(()) } else { Err(String::from("untar failed")) })
        .exit_on_failure();
    std::fs::remove_file("/tmp/image.tar").exit_on_failure();
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