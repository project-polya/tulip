use std::io::{stdout, Write};
use std::path::Path;

use log::*;
use reqwest::{blocking, Url};
use rocksdb::DB;
use serde::*;

use crate::{clear_status, force_get, force_get_json, LogUnwrap};
use crate::settings::{Status, StudentConfig};
use crate::cli::StatusWatch;

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentConfigResponse {
    student: Option<StudentConfig>,
    failure: Option<String>,
}

pub fn handle_request(db: &DB, backend: &str, workdir: &Path, download_only: bool, shellcheck: &Path, id: Option<String>) {
    let server = force_get(db, "server");
    let uuid = force_get(db, "uuid");
    let mut status = force_get_json::<Status>(db, "status");
    if let Some(t) = id {
        clear_status(db, &mut status, workdir);
        status.in_progress.as_mut().unwrap().student_id = t;
    }
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
            .and_then(|x| x.error_for_status())
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
        clear_status(db, &mut status, workdir);
        status.in_progress.replace(new_student.student.take().unwrap());
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
            .and_then(|x| x.error_for_status())
            .exit_on_failure()
            .json::<StudentConfig>()
            .exit_on_failure();
        clear_status(db, &mut status, workdir);
        status.in_progress.replace(ans);
    }
    db.put("status", serde_json::to_vec(&status).exit_on_failure()).exit_on_failure();
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

    info!("shellchecking run script");

    if let Err(e) = std::process::Command::new(shellcheck)
        .arg(student_dir.join(student.run_shell.as_path()))
        .spawn()
        .and_then(|mut x| x.wait())
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("failed with {}", x)) })
    {
        warn!("failed to shellcheck run script: {}", e);
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
        .and_then(|x| x.error_for_status())
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

pub fn skip(db: &DB, force: bool, workdir: &Path) {
    let server = force_get(db, "server");
    let uuid = force_get(db, "uuid");
    let mut status = force_get_json::<Status>(db, "status");
    let student = status.in_progress.as_ref().unwrap_or_else(|| {
        error!("nothing to skip");
        std::process::exit(1);
    });
    let code = blocking::Client::new()
        .put(format!("{}/student/{}/skip", server, student.student_id).parse::<Url>().exit_on_failure())
        .bearer_auth(uuid)
        .send()
        .map(|x| x.status().is_success())
        .unwrap_or_else(|x| {
            error!("{}", x);
            false
        });
    if !code && !force {
        error!("server refused the request");
        std::process::exit(1);
    }
    clear_status(db, &mut status, workdir);
}

pub fn pull(workdir: &Path, id: String, db: &DB, backend: &str, shellcheck: &Path) {
    let status = force_get_json::<Status>(db, "status");
    if status.in_progress.is_some() && !status.submitted {
        error!("current project is not submitted");
        std::process::exit(1);
    }
    handle_request(db, backend, workdir, true, shellcheck, Some(id));
}

pub fn auto_current(workdir: &Path, db: &DB, nutshell: &Path, tmp_size: Option<usize>, mount_point: &Path, shellcheck: &Path, editor: &str) {
    let mut status = force_get_json::<Status>(db, "status");
    if status.in_progress.is_none() {
        error!("No current project");
        std::process::exit(1);
    }
    if !status.image {
        error!("No current image");
        std::process::exit(1);
    }
    let mut mount = true;
    if status.mount.is_some() {
        let mut result = String::new();
        print!("Already mount, re-init the overlay? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        mount = "y" == result.trim().to_ascii_lowercase();
        if mount {
            crate::overlay::handle_destroy(db, workdir);
        }
    }
    if mount {
        crate::overlay::handle(
            db, workdir, nutshell, false, false, mount_point, tmp_size, false
        );
    }
    status = force_get_json::<Status>(db, "status");
    info!("overlay intialized");

    {
        let mut result = String::new();
        print!("Enter the current overlay? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        if "y" == result.trim().to_ascii_lowercase() {
            let code = crate::build::build_nspawn(db, &status, workdir, false, false)
                .spawn()
                .exit_on_failure()
                .wait()
                .exit_on_failure();
            info!("nspawn {}", code);
        }
    }

    {
        let mut result = String::new();
        print!("View the build script? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        if "y" == result.trim().to_ascii_lowercase() {
            crate::status::handle(db, StatusWatch::EditBuildScript {
                editor: editor.to_string(),
                shellcheck: shellcheck.to_path_buf()
            }, workdir);
        }
    }

    let mut build = true;
    if status.built {
        let mut result = String::new();
        print!("Already build, re-build? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        build = "y" == result.trim().to_ascii_lowercase();
    }

    if build {
        crate::build::handle(db, true, workdir);
    }

    {
        let mut result = String::new();
        print!("Enter the sandboxed overlay? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        if "y" == result.trim().to_ascii_lowercase() {
            let code = crate::build::build_nspawn(db, &status, workdir, false, true)
                .spawn()
                .exit_on_failure()
                .wait()
                .exit_on_failure();
            info!("nspawn {}", code);
        }
    }

    {
        let mut result = String::new();
        print!("View the run script? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        if "y" == result.trim().to_ascii_lowercase() {
            crate::status::handle(db, StatusWatch::EditRunScript {
                editor: editor.to_string(),
                shellcheck: shellcheck.to_path_buf()
            }, workdir);
        }
    }

    {
        let mut result = String::new();
        print!("Start running? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        if "y" == result.trim().to_ascii_lowercase() {
            crate::run::run(db, workdir, false);
        }
    }

    {
        let mut result = String::new();
        print!("Enter the firejailed overlay? [Y/n] ");
        std::io::stdout().flush().exit_on_failure();
        std::io::stdin().read_line(&mut result).exit_on_failure();
        if "y" == result.trim().to_ascii_lowercase() {
            let config = force_get_json(db, "config");
            let code = crate::run::build_firejail(mount_point, &config, true, workdir)
                .spawn()
                .exit_on_failure()
                .wait()
                .exit_on_failure();
            info!("nspawn {}", code);
        }
    }

}