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
    pub command: SubCommand
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
    }
}
