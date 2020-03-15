use rocksdb::DB;
use crate::{force_get_json, force_get, LogUnwrap};
use crate::settings::{Status, StudentConfig};
use std::path::{Path, PathBuf};
use log::*;
use reqwest::Url;
use serde::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentConfigResponse {
    student: Option<StudentConfig>,
    failure: Option<String>
}

pub fn handle_request(db: &DB, backend: &str, workdir: &Path, download_only: bool) {
    let server = force_get(db, "server");
    let uuid = force_get(db, "uuid");
    let mut status = force_get_json::<Status>(db, "status");
    if !download_only {
        if !status.submitted && status.in_progress.is_some() {
            error!("current project not submitted, exiting");
            std::process::exit(1);
        }
        status.comment.clear();
        status.graded = None;
        status.submitted = false;
        status.in_progress = None;
        status.stderr = None;
        status.stdout = None;
        let mut new_student = reqwest::blocking::Client::new()
            .get(format!("{}/next", server).parse::<Url>().exit_on_failure())
            .bearer_auth(uuid.as_str())
            .send()
            .and_then(|x| x.json::<StudentConfigResponse>()).exit_on_failure();
        if let Some(f) = new_student.failure {
            error!("failed to get next student: {}", f);
            std::process::exit(1);
        }
        status.in_progress.replace(new_student.student.take().unwrap());

        // TODO: FIXME
    } else {
        if status.in_progress.is_none() {
            error!("current project not existing, exiting");
            std::process::exit(1);
        }
        let ans = reqwest::blocking::Client::new()
            .get(format!("{}/student/{}/info", server, status.in_progress.as_ref().unwrap().student_id.as_str())
                .parse::<Url>().exit_on_failure())
            .bearer_auth(uuid.as_str())
            .send()
            .exit_on_failure()
            .json::<StudentConfig>()
            .exit_on_failure();
        status.in_progress.replace(ans);
    }
    db.put("status", serde_json::to_vec(&status).exit_on_failure()).exit_on_failure();
    crate::overlay::handle_destroy(db, workdir);
    match std::fs::remove_dir_all(workdir.join("student")) {
        Ok(()) => info!("student dir deleted!"),
        Err(e) => error!("failed to delete student dir: {}", e)
    };
    let auth = format!("Authorization: Bearer {}", uuid.as_str());
    let uid = status.in_progress.as_ref().unwrap().student_id.as_str();
    match backend {
        "wget" => {
            std::process::Command::new("wget")
                .arg("--show-progress")
                .arg("-O")
                .arg(format!("/tmp/{}", uid))
                .arg("--header")
                .arg(auth)
                .arg(format!("{}/student/{}/tar", server, uid))
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x|x.to_string())
                .and_then(|x| if x.success() {Ok(())} else {Err(format!("wget failed with: {}", x))})
                .exit_on_failure();
        },
        "aria2c" => {
            std::process::Command::new("aria2c")
                .arg("--optimize-concurrent-downloads")
                .arg("--auto-file-renaming=false")
                .arg("--dir")
                .arg("/tmp")
                .arg("-o")
                .arg(uid)
                .arg("--header")
                .arg(auth)
                .arg(format!("{}/student/{}/tar", server, uid))
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x|x.to_string())
                .and_then(|x| if x.success() {Ok(())} else {Err(format!("aria2c failed with: {}", x))})
                .exit_on_failure();
        }
        _ => unreachable!()
    }

    std::fs::create_dir_all(workdir.join("student")).exit_on_failure();

    std::process::Command::new("tar")
        .arg("-C")
        .arg(workdir.join("student").canonicalize().exit_on_failure())
        .arg("-xf")
        .arg(format!("/tmp/{}", uid))
        .spawn()
        .exit_on_failure()
        .wait()
        .map_err(|x|x.to_string())
        .and_then(|x| if x.success() {Ok(())} else {Err(format!("aria2c failed with: {}", x))})
        .exit_on_failure();
    info!("student information synced");
}