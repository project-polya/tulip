use rocksdb::DB;

use crate::{LogUnwrap, force_get, force_get_json};
use crate::settings::{Status, Config};
use reqwest::Url;
use log::*;
use std::path::Path;
use serde::*;
pub fn handle(force: bool, db: &DB, backend: &str, workdir: &Path) {
    let server = force_get(db, "server");

    let uuid = force_get(db, "uuid");

    let mut status = force_get_json::<Status>(db, "status");

    if status.image && !force {
        error!("image existed, exiting...");
    }
    let request_url : Url = format!("{}/image.tar", server).parse().exit_on_failure();
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
                .map_err(|x|x.to_string())
                .and_then(|mut x|x.wait().map_err(|x|x.to_string()))
                .and_then(|x| if x.success() {Ok(())} else {Err(String::from("download failed"))})
                .exit_on_failure();
        },
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
                .map_err(|x|x.to_string())
                .and_then(|mut x|x.wait().map_err(|x|x.to_string()))
                .and_then(|x| if x.success() {Ok(())} else {Err(String::from("download failed"))})
                .exit_on_failure();
        },
        _ => unreachable!()
    }
    std::process::Command::new("sudo")
        .arg("tar")
        .arg("-C")
        .arg(std::fs::canonicalize(workdir).exit_on_failure())
        .arg("-xf")
        .arg("/tmp/image.tar")
        .spawn()
        .map_err(|x|x.to_string())
        .and_then(|mut x|x.wait().map_err(|x|x.to_string()))
        .and_then(|x| if x.success() {Ok(())} else {Err(String::from("untar failed"))})
        .exit_on_failure();
    std::fs::remove_file("/tmp/image.tar").exit_on_failure();
    status.image = true;
    db.put(b"status", serde_json::to_vec(&status).exit_on_failure().as_mut_slice()).exit_on_failure();
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
        .and_then(|x|x.json::<ConfigResponse>())
        .exit_on_failure();
    db.put(b"config", serde_json::to_vec(&config.config).exit_on_failure().as_mut_slice()).exit_on_failure();
}