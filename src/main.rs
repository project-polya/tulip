use std::io::Write;
use log::*;
use crate::settings::*;
mod cli;
mod init;
mod settings;
use structopt::StructOpt;
use crate::cli::Opt;
use std::path::{Path, PathBuf};
mod clean_all;
use rocksdb::DB;
use std::fmt::{Debug, Display};

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
    std::env::set_var("TULIP_LOG_LEVEL", opt.log_level());
    pretty_env_logger::init_custom_env("TULIP_LOG_LEVEL");
    debug!("tulip invoked with {:#?}", opt);
    must_sudo();
    match opt {
        Opt::Init { tulip_dir, server, token, force, .. } => {
            create_workdir(tulip_dir.as_path());
            let db = tulip_dir.join("meta");
            init::handle(tulip_dir.as_path(), server.as_str(), token.as_str(), &init_db(db.as_path()), force);
        }
        Opt::CleanAll { tulip_dir, force,.. } => {
            let db = tulip_dir.join("meta");
            let res = clean_all::handle_clean(tulip_dir.as_path(), &init_db(db.as_path()));
            if !res && !force {
                error!("clean up failed");
                std::process::exit(1);
            }
            else if !res {
                warn!("clearing failed in the clean way. Fine, let us do it in the dirty way");
                clean_all::handle_dirty(tulip_dir.as_path());
            };
        }
    }
}
