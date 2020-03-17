use std::io::{stdout, Write};
use std::path::Path;

use log::*;
use reqwest::{blocking, Url};
use rocksdb::DB;
use serde::*;

use crate::{force_get, force_get_json, LogUnwrap};
use crate::settings::{Status, StudentConfig};

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentConfigResponse {
    student: Option<StudentConfig>,
    failure: Option<String>,
}

pub fn handle_request(db: &DB, backend: &str, workdir: &Path, download_only: bool, shellcheck: &Path) {
    let server = force_get(db, "server");
    let uuid = force_get(db, "uuid");
    let mut status = force_get_json::<Status>(db, "status");
    if !download_only {
        if !status.submitted && status.in_progress.is_some() {
            error!("current project not submitted, exiting");
            std::process::exit(1);
        }
        status = Status {
            mount: status.mount,
            built: false,
            graded: None,
            comment: None,
            in_progress: None,
            submitted: false,
            image: false,
            mark: false,
            stdout: None,
            stderr: None,
            build_stdout: None,
            build_stderr: None,
        };
        let mut new_student = reqwest::blocking::Client::new()
            .get(format!("{}/next", server).parse::<Url>().exit_on_failure())
            .bearer_auth(uuid.as_str())
            .send()
            .map_err(|x| x.to_string())
            .and_then(|x| {
                let text = x.text().map_err(|x| x.to_string());
                debug!("remote responce: {:#?}", text);
                text.and_then(|mut x|
                    simd_json::serde::from_str::<StudentConfigResponse>(x.as_mut_str())
                        .map_err(|x| x.to_string()))
            }).exit_on_failure();
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
    let student = status.in_progress.as_ref().unwrap();
    match backend {
        "wget" => {
            std::process::Command::new("wget")
                .arg("--show-progress")
                .arg("-O")
                .arg(format!("/tmp/{}", student.student_id.as_str()))
                .arg("--header")
                .arg(auth)
                .arg(format!("{}/student/{}/tar", server, student.student_id.as_str()))
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("wget failed with: {}", x)) })
                .exit_on_failure();
        }
        "aria2c" => {
            std::process::Command::new("aria2c")
                .arg("--optimize-concurrent-downloads")
                .arg("--auto-file-renaming=false")
                .arg("--dir")
                .arg("/tmp")
                .arg("-o")
                .arg(student.student_id.as_str())
                .arg("--header")
                .arg(auth)
                .arg(format!("{}/student/{}/tar", server, student.student_id.as_str()))
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("aria2c failed with: {}", x)) })
                .exit_on_failure();
        }
        _ => unreachable!()
    }

    let student_dir = workdir.join("student");

    std::fs::create_dir_all(student_dir.as_path()).exit_on_failure();

    std::process::Command::new("tar")
        .arg("-C")
        .arg(workdir.join("student").canonicalize().exit_on_failure())
        .arg("-xf")
        .arg(format!("/tmp/{}", student.student_id.as_str()))
        .spawn()
        .exit_on_failure()
        .wait()
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("aria2c failed with: {}", x)) })
        .exit_on_failure();

    info!("shellchecking build script");

    if let Err(e) = std::process::Command::new(shellcheck)
        .arg(student_dir.join(student.build_shell.as_path()))
        .spawn()
        .and_then(|mut x| x.wait())
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("failed with {}", x)) })
    {
        warn!("failed to shellcheck build script: {}", e);
    };

    info!("shellchecking build script");

    if let Err(e) = std::process::Command::new(shellcheck)
        .arg(student_dir.join(student.run_shell.as_path()))
        .spawn()
        .and_then(|mut x| x.wait())
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("failed with {}", x)) })
    {
        warn!("failed to shellcheck build script: {}", e);
    };

    info!("student information synced");

    if !student.notification.is_empty() {
        info!("student notification:\n{}", student.notification);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SubmissionResponse {
    failure: Option<String>
}

pub fn handle_submit(db: &DB, r#override: bool) {
    let server = force_get(db, "server");
    let uuid = force_get(db, "uuid");
    let mut status = force_get_json::<Status>(db, "status");
    if status.in_progress.is_none() {
        println!("no current project");
        std::process::exit(1);
    }
    println!("status: {}", serde_json::to_string_pretty(&status).exit_on_failure());
    print!("Are you sure to submit? [Y/n] ");
    stdout().flush().exit_on_failure();
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).exit_on_failure();
    if "y" != buffer.trim() && "Y" != buffer.trim() { return; }
    blocking::Client::new()
        .put(format!("{}/student/{}/grades", server,
                     status.in_progress.as_ref().unwrap().student_id).parse::<Url>().exit_on_failure())
        .bearer_auth(uuid)
        .json(&status.get_submission(r#override))
        .send()
        .and_then(|x| x.json::<SubmissionResponse>())
        .map_err(|x| x.to_string())
        .and_then(|x| {
            match x.failure {
                None => Ok(()),
                Some(e) => Err(e)
            }
        })
        .exit_on_failure();
    status.submitted = true;
    db.put("status", serde_json::to_string(&status).exit_on_failure()).exit_on_failure();
}