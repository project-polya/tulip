use std::io::{Read, Write};

use reqwest::Url;
use rocksdb::DB;
use serde::*;

use crate::{force_get, force_get_json, LogUnwrap};
use crate::cli::StatusWatch;
use crate::settings::{Config, Status, StudentConfig};

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentList {
    students: Vec<String>
}

pub fn handle(db: &DB, command: StatusWatch) {
    match command {
        StatusWatch::Global => {
            let ans = force_get_json::<Config>(db, "config");
            println!("{:#?}", ans);
        }
        StatusWatch::Current => {
            let ans = force_get_json::<Status>(db, "status");
            println!("{:#?}", ans);
        }
        StatusWatch::Remote => {
            let server = force_get(db, "server");
            let uuid = force_get(db, "uuid");
            let ans = reqwest::blocking::Client::new()
                .get(format!("{}/students", server).parse::<Url>().exit_on_failure())
                .bearer_auth(uuid)
                .send()
                .exit_on_failure()
                .json::<StudentList>()
                .exit_on_failure();
            println!("{:#?}", ans);
        }
        StatusWatch::RemoteID { id } => {
            let server = force_get(db, "server");
            let uuid = force_get(db, "uuid");
            let ans = reqwest::blocking::Client::new()
                .get(format!("{}/student/{}/info", server, id).parse::<Url>().exit_on_failure())
                .bearer_auth(uuid)
                .send()
                .exit_on_failure()
                .json::<StudentConfig>()
                .exit_on_failure();
            println!("{:#?}", ans);
        }
        StatusWatch::EditCurrent { editor } => {
            let mut status = db.get("status")
                .ok().flatten()
                .and_then(|mut x| simd_json::serde::from_slice::<Status>(x.as_mut_slice()).ok())
                .or_else(|| Some(Status::default()))
                .and_then(|x| serde_json::to_vec_pretty(&x).ok())
                .unwrap_or(Vec::new());
            let mut file = tempfile::NamedTempFile::new()
                .exit_on_failure();
            file.write_all(status.as_slice()).exit_on_failure();
            file.flush().exit_on_failure();
            std::process::Command::new(editor)
                .arg(file.path())
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("editor exit with error: {}", x)) })
                .exit_on_failure();
            status.clear();
            file.reopen().exit_on_failure().read_to_end(&mut status).exit_on_failure();
            let status = simd_json::serde::from_slice::<Status>(status.as_mut_slice()).exit_on_failure();
            db.put("status", serde_json::to_string(&status)
                .exit_on_failure()).exit_on_failure();
        }
        StatusWatch::EditGlobal { editor } => {
            let mut config = db.get("config")
                .ok().flatten()
                .and_then(|mut x| simd_json::serde::from_slice::<Config>(x.as_mut_slice()).ok())
                .or_else(|| Some(Config::default()))
                .and_then(|x| serde_json::to_vec_pretty(&x).ok())
                .unwrap_or(Vec::new());
            let mut file = tempfile::NamedTempFile::new()
                .exit_on_failure();
            file.write_all(config.as_slice()).exit_on_failure();
            file.flush().exit_on_failure();
            std::process::Command::new(editor)
                .arg(file.path())
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("editor exit with error: {}", x)) })
                .exit_on_failure();
            config.clear();
            file.reopen().exit_on_failure().read_to_end(&mut config).exit_on_failure();
            let config = simd_json::serde::from_slice::<Config>(config.as_mut_slice()).exit_on_failure();
            db.put("config", serde_json::to_string(&config)
                .exit_on_failure()).exit_on_failure();
        }
    }
}
