use std::path::PathBuf;

use structopt::*;

#[derive(StructOpt, Debug)]
pub enum StatusWatch {
    #[structopt(about = "current project status")]
    Current,
    #[structopt(about = "global configuration")]
    Global,
    #[structopt(about = "remote student lists")]
    Remote,
    #[structopt(about = "remote student info")]
    RemoteID {
        #[structopt(long, short, about = "remote student info")]
        id: String
    },
    #[structopt(about = "edit current project status")]
    EditCurrent {
        #[structopt(short, long, env = "EDITOR", help = "the editor software", default_value = "nano")]
        editor: String
    },
    #[structopt(about = "edit current global settings")]
    EditGlobal {
        #[structopt(short, long, env = "EDITOR", help = "the editor software", default_value = "nano")]
        editor: String
    },
}

#[derive(StructOpt, Debug)]
pub enum Sandbox {
    #[structopt(about = "enter the systemd-nspawn sandbox")]
    SystemdNspawn {
        #[structopt(long, help = "rsync the student data")]
        rsync: bool,
        #[structopt(long, help = "enter without the global config")]
        without_config: bool,
    },
    #[structopt(about = "enter the firejail sandbox")]
    Firejail {
        #[structopt(long, help = "enter without the global config")]
        without_config: bool,
    },
}

#[derive(StructOpt, Debug)]
pub struct Opt {
    #[structopt(short, long, about = "the log level", env = "TULIP_LOG_LEVEL", default_value = "info",
    possible_values = & ["error", "trace", "info", "debug", "off", "warn"])]
    pub log_level: String,
    #[structopt(long, about = "the work directory of tulip", env = "TULIP_DIR", default_value = ".tulip")]
    pub tulip_dir: PathBuf,
    #[structopt(subcommand)]
    pub command: SubCommand,
    #[structopt(long, about = "path to nutshell binary", env = "NUTSHELL_BIN", default_value = "nutshell")]
    pub nutshell: PathBuf,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    #[structopt(about = "register this client")]
    Register {
        #[structopt(short, long, about = "the server address", env = "TULIP_SERVER")]
        server: String,
        #[structopt(short, long, about = "ssh username", env = "TULIP_TOKEN")]
        token: String,
        #[structopt(long, help = "force to register a new uuid")]
        force: bool,
    },
    #[structopt(about = "pull the base image")]
    PullImage {
        #[structopt(long, help = "force to renew the current image")]
        force: bool,
        #[structopt(short, long, help = "backend downloader", default_value = "wget", possible_values = & ["wget", "aria2c"])]
        backend: String,
        #[structopt(long, help = "Use this if you have already untar an image on your own")]
        local_set: bool,
    },
    #[structopt(about = "unregister the client and clean up local environment")]
    CleanAll {
        #[structopt(long, help = "force to remove all local data even without successful communication")]
        force: bool
    },
    #[structopt(about = "see the current status")]
    Status {
        #[structopt(subcommand)]
        command: StatusWatch,
    },
    #[structopt(about = "refresh the global config")]
    RefreshConfig,

    #[structopt(about = "initialize the overlay filesystem")]
    InitOverlay {
        #[structopt(short, long, help = "print a mount result")]
        print_result: bool,
        #[structopt(short, long, help = "enter a shell of systemd-nspawn after initialization")]
        shell: bool,
        #[structopt(short, long, about = "the diretory to mount the root", default_value = "/mnt")]
        mount_dir: PathBuf,
        #[structopt(short, long, about = "when specified, a new tmpfs with the given size will mount in the root")]
        tmp_size: Option<usize>,
        #[structopt(long, help = "force to mount overlay even if there is a record in the database. this can be useful if you want to recover the progress after reboot")]
        force: bool,
    },
    #[structopt(about = "delete the current overlay system")]
    DestroyOverlay,
    #[structopt(about = "Give a grade to the student")]
    Grade {
        #[structopt(short, long, help = "the score")]
        score: usize,
        #[structopt(long, about = "allow override existing score")]
        r#override: bool,
    },
    #[structopt(about = "Open the comment editor")]
    Comment {
        #[structopt(short, long, env = "EDITOR", help = "the editor software", default_value = "nano")]
        editor: String
    },
    #[structopt(about = "fetch student project")]
    Fetch {
        #[structopt(short, long, help = "backend downloader", default_value = "wget", possible_values = & ["wget", "aria2c"])]
        backend: String,
        #[structopt(short, long, about = "do not request next task, only sync current project")]
        download_only: bool,
        #[structopt(short, long, help = "shellcheck path", env = "SHELL_CHECK_BIN", default_value = "shellcheck")]
        shellcheck: PathBuf,
    },
    #[structopt(about = "build the current project")]
    Build {
        #[structopt(long, about = "rebuild the project")]
        rebuild: bool
    },

    #[structopt(about = "build the current project")]
    Run {
        #[structopt(long, about = "force to run without build")]
        without_build: bool
    },

    #[structopt(about = "edit current global settings")]
    Submit {
        #[structopt(long, about = "allow overriding existing submission")]
        r#override: bool
    },
    #[structopt(about = "clear the current project")]
    Clear,
    #[structopt(about = "manually enter the sandbox")]
    EnterSandbox {
        #[structopt(subcommand)]
        command: Sandbox,
    },
}
