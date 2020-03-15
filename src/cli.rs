use structopt::*;
use std::path::PathBuf;
#[derive(StructOpt, Debug)]
pub struct Opt {
    #[structopt(short, long, about="the log level", env="TULIP_LOG_LEVEL", default_value="trace",
    possible_values=&["error", "trace", "info", "debug", "off", "warn"])]
    pub log_level: String,
    #[structopt(long, about="the work directory of tulip", env="TULIP_DIR", default_value=".tulip")]
    pub tulip_dir: PathBuf,
    #[structopt(subcommand)]
    pub command: SubCommand,
    #[structopt(long, about="path to nutshell binary", env="NUTSHELL_BIN", default_value="nutshell")]
    pub nutshell: PathBuf,
}
#[derive(StructOpt, Debug)]
pub enum SubCommand {
    #[structopt(about="register this client")]
    Register {
        #[structopt(short, long, about="the server address", env="TULIP_SERVER")]
        server: String,
        #[structopt(short, long, about="ssh username", env="TULIP_TOKEN")]
        token: String,
        #[structopt(long, help="force to register a new uuid")]
        force: bool
    },
    #[structopt(about="pull the base image")]
    PullImage {
        #[structopt(long, help="force to renew the current image")]
        force: bool,
        #[structopt(short, long, help="backend downloader", default_value="wget", possible_values=&["wget", "aria2c"])]
        backend: String
    },
    #[structopt(about="unregister the client and clean up local environment")]
    CleanAll {
        #[structopt(long, help="force to remove all local data even without successful communication")]
        force: bool
    },
    #[structopt(about="see the current status")]
    Status {
        #[structopt(short, long, help="also check the current global config")]
        global: bool
    },
    #[structopt(about="refresh the global config")]
    RefreshConfig,
    #[structopt(about="initialize the overlay filesystem")]
    InitOverlay {
        #[structopt(short, long, help="print a mount result")]
        print_result: bool,
        #[structopt(short, long, help="enter a shell of systemd-nspawn after initialization")]
        shell: bool,
        #[structopt(short, long, about="the diretory to mount the root", default_value="/mnt")]
        mount_dir: PathBuf,
        #[structopt(short, long, about="when specified, a new tmpfs with the given size will mount in the root")]
        tmp_size: Option<usize>,
    }
}
