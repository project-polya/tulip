use rocksdb::DB;
use crate::cli::StatusWatch;
use crate::{force_get_json, force_get, LogUnwrap};
use crate::settings::{Config, Status, StudentConfig};

use serde::*;
use reqwest::Url;

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
        },
        StatusWatch::RemoteID { id} => {
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
    }
}