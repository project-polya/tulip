use std::path::Path;
use rocksdb::DB;
use log::*;

use reqwest::Url;
use serde::*;
use crate::LogUnwrap;
use crate::clean_all::handle_clean;
use crate::settings::Status;


#[repr(transparent)]
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResult {
    token: String
}

pub fn handle(tulip_dir: &Path, server: &str, token: &str, db: &DB, force: bool) {
    if let Ok(Some(current)) = db.get("uuid") {
        let current_uuid = String::from_utf8_lossy(current.as_ref());
        if force {
            warn!("already inited with {}, but I will do it anyway", current_uuid);
            if !handle_clean(tulip_dir.as_ref(), db) {
                error!("clean failed");
                std::process::exit(0);
            }
        } else {
            error!("already inited with {}, exiting", current_uuid);
            std::process::exit(1);
        }
    }
    let mut seed = [0u8; 16];
    botan::RandomNumberGenerator::new().and_then(|x|x.fill(&mut seed))
        .unwrap_or_else(|e| {
            error!("{:?}", e);
            std::process::exit(1)
        });
    let config = argon2::Config::default();
    let hash = argon2::hash_encoded(token.as_bytes(), &seed, &config).exit_on_failure();
    let register = reqwest::blocking::Client::new()
        .get(format!("{}/register", server).parse::<Url>().exit_on_failure())
        .bearer_auth(hash)
        .send()
        .and_then(|res|res.json::<RegisterResult>())
        .exit_on_failure();
    db.put("uuid", register.token.as_bytes()).exit_on_failure();
    db.put("server", server).exit_on_failure();
    info!("registered as {}", register.token);
    db.put("status", serde_json::to_string(&Status::default()).exit_on_failure()).exit_on_failure();
}