use rocksdb::DB;

use crate::LogUnwrap;
use crate::settings::Status;
use reqwest::Url;
use log::*;
use std::path::Path;

pub fn handle(force: bool, db: &DB, backend: &str, workdir: &Path) {
    let server = db.get("server")
        .map_err(|x| x.to_string())
        .and_then(|x| {
            match x.map(|x| String::from_utf8_lossy(x.as_slice()).to_string()) {
                Some(e) => Ok(e),
                None => Err(String::from("unable to get server"))
            }
        })
        .exit_on_failure();

    let uuid = db.get("uuid")
        .map_err(|x| x.to_string())
        .and_then(|x| {
            match x.map(|x| String::from_utf8_lossy(x.as_slice()).to_string()) {
                Some(e) => Ok(e),
                None => Err(String::from("unable to get uuid"))
            }
        })
        .exit_on_failure();

    let status = db.get("status")
        .map_err(|x| x.to_string())
        .and_then(|x| {
            let parsed = x.and_then(|mut x| simd_json::serde::from_slice::<Status>(x.as_mut_slice()).ok());
            match parsed {
                Some(e) => Ok(e),
                None => Err(String::from("unable to get status"))
            }
        })
        .exit_on_failure();

    if status.image && !force {
        error!("image existed, exiting...");
    }
    let request_url : Url = format!("{}/image.tar", server).parse().exit_on_failure();
    let auth = format!("'Authorization: Bearer {}'", uuid);
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
                .arg("--optimize-concurrent-downloads ")
                .arg("-o")
                .arg("/tmp/image.tar")
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
        .arg("-xf")
        .arg("-C")
        .arg(workdir)
        .spawn()
        .map_err(|x|x.to_string())
        .and_then(|mut x|x.wait().map_err(|x|x.to_string()))
        .and_then(|x| if x.success() {Ok(())} else {Err(String::from("untar failed"))})
        .exit_on_failure();

}