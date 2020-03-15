use log::*;
use crate::settings::*;
mod cli;
mod register;
mod settings;
use structopt::StructOpt;
use crate::cli::{Opt, SubCommand};
use std::path::Path;
mod clean_all;
mod overlay;
mod student;
use rocksdb::DB;
use std::fmt::{Debug, Display};

mod pull_image;
fn force_get(db: &DB, key: &str) -> String {
    db.get(key)
        .map_err(|x| x.to_string())
        .and_then(|x| {
            match x.map(|x| String::from_utf8_lossy(x.as_slice()).to_string()) {
                Some(e) => Ok(e),
                None => Err(format!("unable to get {}", key))
            }
        })
        .exit_on_failure()
}

fn force_get_json<'a, T : serde::Deserialize<'a>>(db: &DB, key: &str) -> T {
    static mut BUFFER : Vec<u8> = Vec::new();
    unsafe {
        BUFFER.clear();
        BUFFER.append(&mut db.get(key).exit_on_failure().unwrap_or(vec![]));
        match simd_json::serde::from_slice::<T>(BUFFER.as_mut_slice()).ok() {
            Some(res) => res,
            None => {
                error!("unable to get {}", key);
                std::process::exit(1)
            }
        }
    }
}

fn init_db(path: &Path) -> DB {
    let db = DB::open_default(path).unwrap_or_else(|x| {
        error!("DATABASE_ERROR: {}", x);
        std::process::exit(2);
    });
    debug!("database initialized");
    db
}

fn must_sudo() {
    let check = std::env::var("USER")
        .map(|x| "root" == x);
    if let Ok(true) = check {
        debug!("runs as root");
    } else {
        error!("root permission is required");
        std::process::exit(1);
    }
}
fn create_workdir(path: &Path) {
    if let Ok(true) = std::fs::metadata(path).map(|x|x.is_dir()) {
        debug!("workdir is already created");
    } else {
        debug!("trying to create workdir");
        std::fs::create_dir_all(path).unwrap_or_else(|x| {
            error!("unable to create workdir: {}", x);
            std::process::exit(3);
        });
        debug!("workdir created");
    }
}
trait LogUnwrap {
    type Return;
    fn exit_on_failure(self) -> Self::Return;
}
impl<A, B : Debug + Display> LogUnwrap for Result<A, B> {
    type Return = A;

    fn exit_on_failure(self) -> Self::Return {
        match self {
            Ok(_) => self.unwrap(),
            Err(e) => {
                error!("{}", e);
                std::process::exit(1)
            }
        }
    }
}
fn main() {
    let opt : Opt = Opt::from_args();
    std::env::set_var("TULIP_LOG_LEVEL", opt.log_level.as_str());
    pretty_env_logger::init_custom_env("TULIP_LOG_LEVEL");
    debug!("tulip invoked with {:#?}", opt);

    match opt.command {
        SubCommand::Register { server, token, force } => {
            create_workdir(opt.tulip_dir.as_path());
            let db = opt.tulip_dir.join("meta");
            register::handle(opt.tulip_dir.as_path(), server.as_str(), token.as_str(), &init_db(db.as_path()), force);
        }
        SubCommand::CleanAll { force } => {
            must_sudo();
            let db = opt.tulip_dir.join("meta");
            let res = clean_all::handle_clean(opt.tulip_dir.as_path(), &init_db(db.as_path()));
            if !res && !force {
                error!("clean up failed");
                std::process::exit(1);
            }
            else if !res {
                warn!("clearing failed in the clean way. Fine, let us do it in the dirty way");
                clean_all::handle_dirty(opt.tulip_dir.as_path());
            };
        }
        SubCommand::PullImage { force, backend } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            pull_image::handle(force, &db, backend.as_str(), opt.tulip_dir.as_path());
        },
        SubCommand::Status { global } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let status = force_get_json::<Status>(&db, "status");
            println!("{:#?}", status);
            if global {
                let status = force_get_json::<Config>(&db, "config");
                println!("{:#?}", status);
            }
        }
        SubCommand::RefreshConfig => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let server = force_get(&db, "server");
            let uuid = force_get(&db, "uuid");
            pull_image::refresh_config(server.as_str(), uuid.as_str(), &db);
        }
        SubCommand::InitOverlay { print_result, shell, mount_dir, tmp_size , force} => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            overlay::handle(&db, opt.tulip_dir.as_path(), opt.nutshell.as_path(), print_result, shell, mount_dir.as_path(), tmp_size, force);
        }
        SubCommand::DestroyOverlay => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            overlay::handle_destroy(&db, opt.tulip_dir.as_path());
        },
        SubCommand::Next { backend } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            student::handle_next(&db, backend.as_str(), opt.tulip_dir.as_path());
        }
    }
}
