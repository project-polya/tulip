use std::path::PathBuf;

use structopt::*;

#[derive(StructOpt, Debug)]
pub enum StatusWatch {
    #[structopt(about = "Current project status")]
    Current {
        #[structopt(long, short, help = "Show IO data")]
        io_data: bool
    },
    #[structopt(about = "Global configuration")]
    Global,
    #[structopt(about = "Remote student lists")]
    Remote {
        #[structopt(short, long, help = "Show student detail")]
        detail: bool,
    },
    #[structopt(about = "Remote student info")]
    RemoteID {
        #[structopt(long, short, help = "Remote student info")]
        id: String
    },
    #[structopt(about = "Edit current project status")]
    EditCurrent {
        #[structopt(short, long, env = "EDITOR", help = "The editor software", default_value = "nano")]
        editor: String
    },
    #[structopt(about = "Edit current global settings")]
    EditGlobal {
        #[structopt(short, long, env = "EDITOR", help = "The editor software", default_value = "nano")]
        editor: String
    },
    #[structopt(about = "Edit the build script")]
    EditBuildScript {
        #[structopt(short, long, env = "EDITOR", help = "The editor software", default_value = "nano")]
        editor: String,
        #[structopt(short, long, help = "Shellcheck path", env = "SHELL_CHECK_BIN", default_value = "shellcheck")]
        shellcheck: PathBuf,
    },
    #[structopt(about = "Edit the run script")]
    EditRunScript {
        #[structopt(short, long, env = "EDITOR", help = "The editor software", default_value = "nano")]
        editor: String,
        #[structopt(short, long, help = "Shellcheck path", env = "SHELL_CHECK_BIN", default_value = "shellcheck")]
        shellcheck: PathBuf,
    },
    #[structopt(about = "check local uuid")]
    Uuid,
    #[structopt(about = "check current server")]
    Server {
        #[structopt(short, long, help = "edit server")]
        change_to: Option<String>,
    },
}

#[derive(StructOpt, Debug)]
pub enum Sandbox {
    #[structopt(about = "Enter the systemd-nspawn sandbox")]
    SystemdNspawn {
        #[structopt(long, help = "Rsync the student data")]
        rsync: bool,
        #[structopt(long, help = "Enter without the global config")]
        without_config: bool,
    },
    #[structopt(about = "Enter the firejail sandbox")]
    Firejail {
        #[structopt(long, help = "Enter without the global config")]
        without_config: bool,
    },
}

#[derive(StructOpt, Debug)]
pub struct Opt {
    #[structopt(short, long, help = "the log level", env = "TULIP_LOG_LEVEL", default_value = "info",
    possible_values = & ["error", "trace", "info", "debug", "off", "warn"])]
    pub log_level: String,
    #[structopt(short, long, help = "The work directory of tulip", env = "TULIP_DIR", default_value = ".tulip")]
    pub tulip_dir: PathBuf,
    #[structopt(subcommand)]
    pub command: SubCommand,
    #[structopt(short, long, help = "Path to nutshell binary", env = "NUTSHELL_BIN", default_value = "nutshell")]
    pub nutshell: PathBuf,
}

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    #[structopt(about = "Register this client")]
    Register {
        #[structopt(short, long, help = "The server address", env = "TULIP_SERVER")]
        server: String,
        #[structopt(short, long, help = "Ssh username", env = "TULIP_TOKEN")]
        token: String,
        #[structopt(long, help = "Force to register a new uuid")]
        force: bool,
    },
    #[structopt(about = "Pull the base image")]
    PullImage {
        #[structopt(long, help = "Force to renew the current image")]
        force: bool,
        #[structopt(short, long, help = "backend downloader", default_value = "wget", possible_values = & ["wget", "aria2c"])]
        backend: String,
        #[structopt(long, help = "Use this if you have already untar an image on your own")]
        local_set: bool,
    },
    #[structopt(about = "Unregister the client and clean up local environment")]
    CleanAll {
        #[structopt(long, help = "Force to remove all local data even without successful communication")]
        force: bool,
        #[structopt(short, long, help = "Keep the image")]
        keep_image: bool,
    },
    #[structopt(about = "See the current status")]
    Status {
        #[structopt(subcommand)]
        command: StatusWatch,
    },
    #[structopt(about = "Refresh the global config")]
    RefreshConfig,

    #[structopt(about = "Initialize the overlay filesystem")]
    InitOverlay {
        #[structopt(short, long, help = "Print a mount result")]
        print_result: bool,
        #[structopt(short, long, help = "Enter a shell of systemd-nspawn after initialization")]
        shell: bool,
        #[structopt(short, long, env = "TULIP_MOUNT_DIR", help = "The diretory to mount the root", default_value = "/mnt")]
        mount_dir: PathBuf,
        #[structopt(short, long, help = "When specified, a new tmpfs with the given size will mount in the root")]
        tmp_size: Option<usize>,
        #[structopt(long, help = "Force to mount overlay even if there is a record in the database. this can be useful if you want to recover the progress after reboot")]
        force: bool,
    },
    #[structopt(about = "Delete the current overlay system")]
    DestroyOverlay,
    #[structopt(about = "Give a grade to the student")]
    Grade {
        #[structopt(short, long, help = "The score")]
        score: usize,
        #[structopt(long, help = "Allow override existing score")]
        r#override: bool,
    },
    #[structopt(about = "Open the comment editor")]
    Comment {
        #[structopt(short, long, env = "EDITOR", help = "The editor software", default_value = "nano")]
        editor: String
    },
    #[structopt(about = "Fetch student project")]
    Fetch {
        #[structopt(short, long, help = "backend downloader", default_value = "wget", possible_values = & ["wget", "aria2c"])]
        backend: String,
        #[structopt(short, long, help = "Do not request next task, only sync current project")]
        download_only: bool,
        #[structopt(short, long, help = "Shellcheck path", env = "SHELL_CHECK_BIN", default_value = "shellcheck")]
        shellcheck: PathBuf,
    },
    #[structopt(about = "Pull the target student project")]
    Pull {
        #[structopt(short, long, help = "backend downloader", default_value = "wget", possible_values = & ["wget", "aria2c"])]
        backend: String,
        #[structopt(short, long, help = "Student ID")]
        id: String,
        #[structopt(short, long, help = "Shellcheck path", env = "SHELL_CHECK_BIN", default_value = "shellcheck")]
        shellcheck: PathBuf,
    },
    #[structopt(about = "Auto run the current project")]
    AutoCurrent {
        #[structopt(short, long, help = "When specified, a new tmpfs with the given size will mount in the root")]
        tmp_size: Option<usize>,
        #[structopt(short, long, help = "The diretory to mount the root", default_value = "/mnt")]
        mount_point: PathBuf,
        #[structopt(short, long, help = "Shellcheck path", env = "SHELL_CHECK_BIN", default_value = "shellcheck")]
        shellcheck: PathBuf,
        #[structopt(short, long, env = "EDITOR", help = "The editor software", default_value = "nano")]
        editor: String,
        #[structopt(short, long, help = "Reader path", env = "TULIP_REPORT_READER", default_value = "xdg-open")]
        reader: PathBuf,
    },
    #[structopt(about = "Build the current project")]
    Build {
        #[structopt(long, help = "Rebuild the project")]
        rebuild: bool
    },

    #[structopt(about = "Build the current project")]
    Run {
        #[structopt(long, help = "Force to run without build")]
        without_build: bool
    },

    #[structopt(about = "Edit current global settings")]
    Submit {
        #[structopt(long, help = "Allow overriding existing submission")]
        r#override: bool
    },
    #[structopt(about = "Clear the current project")]
    Clear,
    #[structopt(about = "Manually enter the sandbox")]
    EnterSandbox {
        #[structopt(subcommand)]
        command: Sandbox,
    },
    #[structopt(about = "Mark the current project")]
    Mark {
        #[structopt(short, long, help = "Remove the mark")]
        remove: bool,
    },

    #[structopt(about = "Skip the current project")]
    Skip {
        #[structopt(long, help = "Force skipping even without a proper response from the server")]
        force: bool,
    },
    #[structopt(about = "Read the report")]
    Report {
        #[structopt(short, long, help = "Reader path", env = "TULIP_REPORT_READER", default_value = "xdg-open")]
        reader: PathBuf,
    },

}
