use std::fmt::{Debug, Display};
use std::io::{Read, Write};
use std::path::Path;

use log::*;
use mimalloc::MiMalloc;
use rocksdb::DB;
use structopt::StructOpt;

use crate::cli::{Opt, Sandbox, SubCommand};
use crate::settings::*;

mod cli;
mod register;
mod settings;
mod clean_all;
mod overlay;
mod student;
mod status;
mod pull_image;
mod build;
mod run;

#[global_allocator]
static ALLOC: MiMalloc = MiMalloc;

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

fn force_get_json<'a, T: serde::Deserialize<'a>>(db: &DB, key: &str) -> T {
    static mut BUFFER: Vec<u8> = Vec::new();
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
    if let Ok(true) = std::fs::metadata(path).map(|x| x.is_dir()) {
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

impl<A, B: Debug + Display> LogUnwrap for Result<A, B> {
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
    let opt: Opt = Opt::from_args();
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
            } else if !res {
                warn!("clearing failed in the clean way. Fine, let us do it in the dirty way");
                clean_all::handle_dirty(opt.tulip_dir.as_path());
            };
        }
        SubCommand::PullImage { force, backend, local_set } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            if local_set {
                pull_image::handle_local(&db, opt.tulip_dir.as_path());
            } else {
                pull_image::handle(force, &db, backend.as_str(), opt.tulip_dir.as_path());
            }
        }
        SubCommand::Status { command } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            status::handle(&db, command);
        }
        SubCommand::RefreshConfig => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let server = force_get(&db, "server");
            let uuid = force_get(&db, "uuid");
            pull_image::refresh_config(server.as_str(), uuid.as_str(), &db);
        }
        SubCommand::InitOverlay { print_result, shell, mount_dir, tmp_size, force } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            overlay::handle(&db, opt.tulip_dir.as_path(), opt.nutshell.as_path(), print_result, shell, mount_dir.as_path(), tmp_size, force);
        }
        SubCommand::DestroyOverlay => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            overlay::handle_destroy(&db, opt.tulip_dir.as_path());
        }
        SubCommand::Fetch { backend, download_only , shellcheck} => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            student::handle_request(&db, backend.as_str(), opt.tulip_dir.as_path(), download_only, shellcheck.as_path());
        }
        SubCommand::Grade { score, r#override } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let mut status = force_get_json::<Status>(&db, "status");
            if status.in_progress.is_none() {
                error!("no current project");
                std::process::exit(1);
            }
            if status.graded.is_some() && !r#override {
                error!("grading exists");
                std::process::exit(1);
            }
            let config = force_get_json::<Config>(&db, "config");
            if config.max_grade < score {
                error!("score out of range");
                std::process::exit(1);
            }
            status.graded.replace(score);
            db.put("status", serde_json::to_string(&status)
                .exit_on_failure()).exit_on_failure();
        }
        SubCommand::Comment { editor } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let mut status = force_get_json::<Status>(&db, "status");
            if status.in_progress.is_none() {
                error!("no current project");
                std::process::exit(1);
            }
            let mut file = tempfile::NamedTempFile::new()
                .exit_on_failure();
            if status.comment.is_some() {
                file.write_all(status.comment.as_ref().unwrap().as_bytes()).exit_on_failure();
            }
            file.flush().exit_on_failure();
            std::process::Command::new(editor)
                .arg(file.path())
                .spawn()
                .exit_on_failure()
                .wait()
                .map_err(|x| x.to_string())
                .and_then(|x| if x.success() { Ok(()) } else { Err(format!("editor exit with error: {}", x)) })
                .exit_on_failure();
            let mut buf = String::new();
            file.reopen().exit_on_failure().read_to_string(&mut buf).exit_on_failure();
            if !buf.is_empty() { status.comment.replace(buf); }
            db.put("status", serde_json::to_string(&status)
                .exit_on_failure()).exit_on_failure();
        }
        SubCommand::Build { rebuild } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            build::handle(&db, rebuild, opt.tulip_dir.as_path());
        }
        SubCommand::Run { without_build } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            run::run(&db, opt.tulip_dir.as_path(), without_build);
        }
        SubCommand::Submit { r#override } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            student::handle_submit(&db, r#override);
        }
        SubCommand::Clear => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let mut status = force_get_json::<Status>(&db, "status");
            if status.in_progress.is_some() && !status.submitted {
                error!("please submit or skip the current project first");
                std::process::exit(1);
            }
            clear_status(&db, &mut status, opt.tulip_dir.as_path());
        }
        SubCommand::EnterSandbox { command } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let status = force_get_json::<Status>(&db, "status");
            let config = force_get_json::<Config>(&db, "config");
            let mut exec = match command {
                cli::Sandbox::Firejail { without_config } => {
                    let mount_point = status.mount.unwrap_or_else(|| {
                        error!("please mount a overlay first");
                        std::process::exit(1);
                    });
                    run::build_firejail(mount_point.as_path(),
                                        &config, !without_config, opt.tulip_dir.as_path())
                }
                Sandbox::SystemdNspawn { rsync, without_config } => {
                    if status.mount.is_none() {
                        error!("please mount a overlay first");
                        std::process::exit(1);
                    };
                    build::build_nspawn(&db, &status, opt.tulip_dir.as_path(), rsync, !without_config)
                }
            };
            let exiting = exec.spawn()
                .exit_on_failure()
                .wait()
                .exit_on_failure();
            info!("{}", exiting);
        },

        SubCommand::Mark { remove } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            let mut status = force_get_json::<Status>(&db, "status");
            if status.in_progress.is_none() {
                error!("no current project");
                std::process::exit(1);
            }
            status.mark = !remove;
            db.put("status", serde_json::to_string(&status)
                .exit_on_failure()).exit_on_failure();
        },

        SubCommand::Skip { force } => {
            let db = init_db(opt.tulip_dir.join("meta").as_path());
            student::skip(&db, force, opt.tulip_dir.as_path());
        }
    }
}

fn clear_status(db: &DB, status: &mut Status, workdir: &Path) {
    overlay::handle_destroy(&db, workdir);
    *status = Status {
        mount: None,
        built: false,
        graded: None,
        comment: None,
        in_progress: None,
        submitted: false,
        image: status.image,
        mark: false,
        stdout: None,
        stderr: None,
        build_stdout: None,
        build_stderr: None,
    };
    if let Err(e) = std::fs::remove_dir_all(workdir.join("student")) {
        warn!("failed to remove student dir: {}", e);
    }
    db.put("status", serde_json::to_string(status)
        .exit_on_failure()).exit_on_failure();
}