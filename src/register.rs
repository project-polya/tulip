use std::path::Path;

use log::*;
use reqwest::Url;
use ring::rand::SecureRandom;
use rocksdb::DB;
use serde::*;

use crate::clean_all::handle_clean;
use crate::LogUnwrap;
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
            if !handle_clean(tulip_dir.as_ref(), db, false) {
                error!("clean failed");
                std::process::exit(0);
            }
        } else {
            error!("already inited with {}, exiting", current_uuid);
            std::process::exit(1);
        }
    }
    let mut seed = [0u8; 16];
    ring::rand::SystemRandom::new().fill(&mut seed).exit_on_failure();
    let config = argon2::Config::default();
    let hash = argon2::hash_encoded(token.as_bytes(), &seed, &config).exit_on_failure();
    let register = reqwest::blocking::Client::new()
        .post(format!("{}/register", server).parse::<Url>().exit_on_failure())
        .bearer_auth(hash)
        .send()
        .and_then(|x| x.error_for_status())
        .and_then(|res| res.json::<RegisterResult>())
        .exit_on_failure();
    db.put("uuid", register.token.as_bytes()).exit_on_failure();
    db.put("server", server).exit_on_failure();
    info!("registered as {}", register.token);
    db.put("status", serde_json::to_string(&Status::default()).exit_on_failure()).exit_on_failure();
}
