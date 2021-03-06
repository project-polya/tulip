use std::path::PathBuf;

use prettytable::*;
use serde::*;
use serde::export::fmt::Display;
use serde::ser::Error;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EnvPair {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Syscall {
    pub name: String,
    pub permit: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Limit {
    pub mem_limit: Option<usize>,
    pub nofile_limit: Option<usize>,
    pub filesize_limit: Option<usize>,
    pub process_limit: Option<usize>,
    pub sigpending_limit: Option<usize>,
    pub cpu_nums: Option<usize>,

}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Binding {
    pub source: PathBuf,
    pub target: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NSpawnConfig {
    pub pid2: bool,
    pub env: Vec<EnvPair>,
    pub work_path: Option<PathBuf>,
    // in the root
    pub syscall: Vec<Syscall>,
    pub capacity: Vec<String>,
    pub capacity_drop: Vec<String>,
    pub no_new_privileges: bool,
    pub no_network: bool,
    pub limit: Option<Limit>,
    pub shell: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Timeout {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FuntionList {
    pub nou2f: bool,
    pub novideo: bool,
    pub no3d: bool,
    pub noautopulse: bool,
    pub nogroups: bool,
    pub nonewprivs: bool,
    pub nodvd: bool,
    pub nodbus: bool,
    pub nonet: bool,
}


// deterministic-exit-code = true
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FirejailConfig {
    pub timeout: Option<Timeout>,
    pub syscall: Vec<Syscall>,
    pub shell: Option<String>,
    pub nice: Option<usize>,
    pub function: FuntionList,
    pub mac: Option<String>,
    pub dns: Option<Vec<String>>,
    pub nodefault: bool,
    pub allow_debuggers: bool,
    pub limit: Option<Limit>,
    pub capacity: Vec<String>,
    pub capacity_drop: Vec<String>,
    pub with_profile: Option<PathBuf>,
    // relative path based on `.local/tulip/image`
    pub has_x: bool,
    pub env: Vec<EnvPair>,
    pub env_remove: Vec<String>,
    pub whilelist: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub systemd_nspawn: NSpawnConfig,
    pub firejail: FirejailConfig,
    pub notification: String,
    pub max_grade: usize,
    pub stdin: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct StudentConfig {
    pub student_id: String,
    pub build_shell: PathBuf,
    pub run_shell: PathBuf,
    pub notification: String,
    pub report: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Status {
    pub mount: Option<PathBuf>,
    pub built: bool,
    pub graded: Option<usize>,
    pub comment: Option<String>,
    pub in_progress: Option<StudentConfig>,
    pub submitted: bool,
    pub image: bool,
    pub mark: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub build_stdout: Option<String>,
    pub build_stderr: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Submission {
    pub graded: Option<usize>,
    pub comment: Option<String>,
    pub mark: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub build_stdout: Option<String>,
    pub build_stderr: Option<String>,
    pub r#override: bool,
}

trait ToTableItem {
    fn to_table_item(&self) -> Box<dyn Display>;
}


pub fn to_table<'de, T: Serialize>(s: &T) -> Result<Table, Box<impl Error>> {
    serde_json::value::to_value(s)
        .map_err(|x| Box::new(x))
        .and_then(|x| {
            match x {
                Value::Object(e) => {
                    let mut table = Table::new();
                    for (x, y) in e {
                        table.add_row(row![bFy->x, bFb->y.to_table_item()]);
                    }
                    Ok(table)
                }
                _ => Err(
                    Box::new(Error::custom("to table can only be used to struct"))
                )
            }
        })
}


impl<T: Display> ToTableItem for Option<T> {
    fn to_table_item(&self) -> Box<dyn Display> {
        match self {
            None => Box::new("N/A"),
            Some(e) => Box::new(e.to_string())
        }
    }
}

impl ToTableItem for Value {
    fn to_table_item(&self) -> Box<dyn Display> {
        match self {
            Value::String(x) => Box::new(x.to_string()),
            Value::Null => Box::new("N/A"),
            Value::Bool(value) => Box::new(*value),
            Value::Number(t) => Box::new(t.to_string()),
            Value::Array(t) => {
                if t.is_empty() {
                    Box::new("")
                } else {
                    let mut table = Table::new();
                    for i in t {
                        table.add_row(row![i.to_table_item()]);
                    }
                    Box::new(table)
                }
            }
            Value::Object(t) => {
                let mut table = Table::new();
                for (x, y) in t {
                    table.add_row(row![bFr->x, bfg->y.to_table_item()]);
                }
                Box::new(table)
            }
        }
    }
}


impl Status {
    pub fn get_submission(&self, r#override: bool) -> Submission {
        Submission {
            graded: self.graded.clone(),
            comment: self.comment.clone(),
            mark: self.mark,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
            build_stdout: self.build_stderr.clone(),
            build_stderr: self.build_stderr.clone(),
            r#override,
        }
    }
}