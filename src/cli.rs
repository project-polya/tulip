use structopt::*;
use std::path::PathBuf;

#[derive(StructOpt, Debug, )]
pub enum Opt {
    #[structopt(about="init local environment")]
    Init {
        #[structopt(long, about="the work directory of tulip", env="TULIP_DIR", default_value="/root/.tulip")]
        tulip_dir: PathBuf,
        #[structopt(short, long, about="the server address", env="TULIP_SERVER")]
        server: String,
        #[structopt(short, long, about="the log level", env="TULIP_LOG_LEVEL", default_value="trace",
        possible_values=&["error", "trace", "info", "debug", "off", "warn"])]
        log_level: String,
        #[structopt(short, long, about="ssh username", env="TULIP_TOKEN")]
        token: String,
        #[structopt(long, help="force to re-init the environment")]
        force: bool
    },
    CleanAll {
        #[structopt(short, long, about="the log level", env="TULIP_LOG_LEVEL", default_value="trace",
        possible_values=&["error", "trace", "info", "debug", "off", "warn"])]
        log_level: String,
        #[structopt(long, about="the work directory of tulip", env="TULIP_DIR", default_value="/root/.tulip")]
        tulip_dir: PathBuf,
        #[structopt(long, help="force to remove all local data without communication")]
        force: bool
    }
}

impl Opt {
    pub fn log_level(&self) -> &str {
        match self {
            Opt::Init { log_level, ..} => log_level.as_str(),
            Opt::CleanAll { log_level, ..} => log_level.as_str()
        }
    }
}