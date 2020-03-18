use std::io::{Read, Write};

use prettytable::{Row, Table};
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

#[derive(Serialize, Deserialize, Debug)]
pub struct StudentStatus {
    skipped: bool,
    finished: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Submission {
    //pub mark: bool, TODO: FIXME
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
    grades: Submission,
    status: StudentStatus,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DetailResponse {
    pub students: Vec<StudentDetail>
}

pub fn student_table(data: &Vec<StudentDetail>) {
    use prettytable::*;
    // Create the table
    let mut table = Table::new();

    // Add a row per time
    table.add_row(row![bFb->"ID", bFr->"Grade", bFy->"Marked", bFw->"Skipped"]);
    for i in data {
        table.add_row(row![i.student_id.as_str(),
         i.grades.graded.as_ref().map(|x|x.to_string()).unwrap_or_else(String::new).as_str(),
         i.status.skipped.to_string().as_str(),
         i.status.skipped.to_string().as_str()]);
    }

    table.printstd();
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
                println!("{:#?}", ans);
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
    }
}

