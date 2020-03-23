use std::io::{Read, stdin, Write};
use std::path::Path;

use log::*;
use prettytable::*;
use reqwest::Url;
use rocksdb::DB;
use serde::*;

use crate::{force_get, force_get_json, LogUnwrap};
use crate::cli::StatusWatch;
use crate::settings::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentList {
    students: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentStatus {
    skipped: bool,
    finished: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Submission {
    pub mark: Option<bool>,
    // TODO: FIXME
    pub graded: Option<usize>,
    pub comment: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub build_stdout: Option<String>,
    pub build_stderr: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentDetail {
    student_id: String,
    grades: Option<Submission>,
    status: StudentStatus,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DetailResponse {
    pub students: Vec<StudentDetail>
}

pub fn in_progress_table(student: &StudentConfig) -> Table {
    let mut table = Table::new();
    table.add_row(row![bFb->"ID", bFr->student.student_id.as_str()]);
    table.add_row(row![bFb->"Notification", bFr->student.notification.as_str()]);
    table.add_row(row![bFb->"Build Shell", bFr->student.build_shell.to_string_lossy()]);
    table.add_row(row![bFb->"Run Shell", bFr->student.run_shell.to_string_lossy()]);
    table
}

pub fn current_table(status: &Status, io_data: bool) {
    let mut table = Table::new();
    table.add_row(row![bFb->"Mount Point", bFr->status.mount.as_ref().map(|x|x.to_string_lossy().to_string())
        .unwrap_or_else(||String::from("N/A"))]);
    table.add_row(row![bFb->"Built", bFr->status.built]);
    table.add_row(row![bFb->"Grade", bFr->status.graded.map(|x|x.to_string()).unwrap_or_else(|| String::from("N/A"))]);
    table.add_row(row![bFb->"Comment", bFr->status.comment.clone().unwrap_or_else(String::new)]);
    table.add_row(row![bFb->"Submitted", bFr->status.submitted]);
    table.add_row(row![bFb->"Image Ready", bFr->status.image]);
    table.add_row(row![bFb->"Mark", bFr->status.mark]);
    if let Some(student) = &status.in_progress {
        table.add_row(row![bFb->"In Progress", bFr->in_progress_table(student)]);
    }
    if io_data {
        table.add_row(row![bFb->"Stdout", bFr->status.stdout.clone().unwrap_or_else(String::new)]);
        table.add_row(row![bFb->"Stderr", bFr->status.stderr.clone().unwrap_or_else(String::new)]);
        table.add_row(row![bFb->"Build Stdout", bFr->status.build_stdout.clone().unwrap_or_else(String::new)]);
        table.add_row(row![bFb->"Build Stderr", bFr->status.build_stderr.clone().unwrap_or_else(String::new)]);
    }
    table.printstd();
}

pub fn student_table(data: &Vec<StudentDetail>) {

    // Create the table
    let mut table = Table::new();

    // Add a row per time
    table.add_row(row![bFb->"ID", bFr->"Grade", bFy->"Marked", bFw->"Skipped"]);
    for i in data {
        table.add_row(row![i.student_id,
         i.grades.as_ref().and_then(|x| x.graded.as_ref().map(|x|x.to_string())).unwrap_or(String::from("N/A")),
         i.grades.as_ref().and_then(|x| x.mark).unwrap_or(false),
         i.status.skipped]);
    }

    table.printstd();
}

pub fn handle(db: &DB, command: StatusWatch, workdir: &Path) {
    match command {
        StatusWatch::Global => {
            let ans = force_get_json::<Config>(db, "config");
            let a = to_table(&ans).exit_on_failure();
            a.printstd();
        }
        StatusWatch::Current { io_data } => {
            let ans = force_get_json::<Status>(db, "status");
            current_table(&ans, io_data);
        }
        StatusWatch::Remote { detail } => {
            let server = force_get(db, "server");
            let uuid = force_get(db, "uuid");
            let client = reqwest::blocking::Client::new();
            if !detail {
                let ans = client.get(format!("{}/students", server).parse::<Url>().exit_on_failure())
                    .bearer_auth(uuid)
                    .send()
                    .exit_on_failure()
                    .error_for_status()
                    .exit_on_failure()
                    .json::<StudentList>()
                    .exit_on_failure();
                let mut table = Table::new();
                table.add_row(row![bFb->"Student List"]);
                for i in ans.students {
                    table.add_row(row![bFy->i.as_str()]);
                }
                table.printstd();
            } else {
                let ans = client.get(format!("{}/students?detail", server).parse::<Url>().exit_on_failure())
                    .bearer_auth(uuid)
                    .send()
                    .exit_on_failure()
                    .error_for_status()
                    .exit_on_failure()
                    .json::<DetailResponse>()
                    .exit_on_failure();
                student_table(&ans.students);
            }
        }
        StatusWatch::RemoteID { id } => {
            let server = force_get(db, "server");
            let uuid = force_get(db, "uuid");
            let ans = reqwest::blocking::Client::new()
                .get(format!("{}/student/{}/info", server, id).parse::<Url>().exit_on_failure())
                .bearer_auth(uuid)
                .send()
                .exit_on_failure()
                .error_for_status()
                .exit_on_failure()
                .json::<StudentConfig>()
                .exit_on_failure();
            to_table(&ans).exit_on_failure().printstd();
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
        StatusWatch::Uuid => {
            let uuid = force_get(db, "uuid");
            println!("uuid: {}", uuid);
        }
        StatusWatch::Server { change_to } => {
            if let Some(new) = change_to {
                db.put("server", new).exit_on_failure();
            } else {
                let server = force_get(db, "server");
                println!("server: {}", server);
            }
        }
        StatusWatch::EditBuildScript { editor, shellcheck } => {
            let ans = force_get_json::<Status>(db, "status");
            edit_script(editor.as_str(), true, shellcheck.as_path(), &ans, workdir);
        }
        StatusWatch::EditRunScript { editor, shellcheck } => {
            let ans = force_get_json::<Status>(db, "status");
            edit_script(editor.as_str(), false, shellcheck.as_path(), &ans, workdir);
        }
        StatusWatch::ResetSkip { id  } => {
            let server = force_get(db, "server");
            let uuid = force_get(db, "uuid");
            reqwest::blocking::Client::new()
                .delete(format!("{}/student/{}/skip", server, id).parse::<Url>().exit_on_failure())
                .bearer_auth(uuid)
                .send()
                .and_then(|x|x.error_for_status())
                .exit_on_failure();
            info!("target student skipping status reset successfully");
        }
        StatusWatch::ResetGrade { id } => {
            let server = force_get(db, "server");
            let uuid = force_get(db, "uuid");
            reqwest::blocking::Client::new()
                .delete(format!("{}/student/{}/grades", server, id).parse::<Url>().exit_on_failure())
                .bearer_auth(uuid)
                .send()
                .and_then(|x|x.error_for_status())
                .exit_on_failure();
            info!("target student grading status reset successfully");
        }
    }
}

fn edit_script(editor: &str, build_or_run: bool, shellcheck: &Path, status: &Status, workdir: &Path) {
    if status.in_progress.is_none() {
        error!("no current project");
        std::process::exit(1);
    }
    let project = status.in_progress.as_ref().unwrap();
    let path = if build_or_run {
        workdir.join("student").join(project.build_shell.as_path())
    } else {
        workdir.join("student").join(project.run_shell.as_path())
    };
    info!("editing {} with {}", path.display(), editor);
    std::process::Command::new(editor)
        .arg(path.as_path())
        .spawn()
        .and_then(|mut x| x.wait())
        .map_err(|x| x.to_string())
        .and_then(|x| if x.success() { Ok(()) } else { Err(format!("failed with {}", x)) })
        .exit_on_failure();
    print!("Runshell checking? [Y/n] ");
    std::io::stdout().flush().exit_on_failure();
    let mut line = String::new();
    stdin().read_line(&mut line).exit_on_failure();
    if "y" == line.trim().to_ascii_lowercase() {
        match std::process::Command::new(shellcheck)
            .arg(path.as_path())
            .spawn()
            .and_then(|mut x| x.wait())
            .map_err(|x| x.to_string())
            .and_then(|x| if x.success() { Ok(format!("exited with {}", x)) } else { Err(format!("failed with {}", x)) })
        {
            Ok(t) => { info!("{}", t) }
            Err(t) => { error!("{}", t) }
        }
    }
}