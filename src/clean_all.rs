use std::path::Path;
use rocksdb::{DB, Options};
use reqwest::Url;
use log::*;

pub fn handle_clean(workdir: &Path, db: &DB) -> bool {
    let uuid;
    if let Ok(Some(_uuid)) = db.get("uuid") {
        uuid = String::from_utf8_lossy(_uuid.as_slice()).to_string();
        info!("clearing uuid: {}", uuid);
    } else { return false }

    let server;
    if let Ok(Some(_server)) = db.get("server") {
        server = String::from_utf8_lossy(_server.as_slice()).to_string();
        info!("clearing server: {}", server);
    } else { return false }

    let url;

    match format!("{}/revoke", server).parse::<Url>() {
        Ok(_url) => {url = _url; }
        Err(error) => {
            error!("{}", error);
            return false;
        }
    }

    match reqwest::blocking::Client::new()
        .delete(url)
        .bearer_auth(uuid.as_str())
        .send()
        .map_err(|x|x.to_string())
        .and_then(|x| {
            if !x.status().is_success() {
                Err(String::from("unable to revoke"))
            } else { Ok(()) }
        }) {
        Ok(_) => {
            info!("successfully revoked {}", uuid);
        }
        Err(error) => {
            error!("{}", error);
            return false;
        }
    }
    handle_dirty(workdir);
    true
}

pub fn handle_dirty(workdir: &Path) {
    match DB::destroy(&Options::default(), workdir.join("meta").as_path()) {
        Ok(_) => {info!("local database destroyed"); }
        Err(e) => {
            error!("failed to destroy database: {}", e);
        }
    };
    match std::fs::remove_dir_all(workdir.join("data")) {
        Ok(_) => {info!("data dir removed"); }
        Err(e) => {
            error!("failed to remove data dir: {}", e);
        }
    }
}