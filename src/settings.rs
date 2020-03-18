use std::path::PathBuf;

use serde::*;

use crate::status::StudentDetail;

/// By invoke `setup` the client will first drag the image.
/// The image contains a `root.x86_64` together with other files.
/// ```
/// image.tar.lz4:
///   - root.x86_64
///   - otherfiles
/// ```
/// The image will then be untared into the `.local/tulip/image` direcotry.
/// Then it will download a config file.
///
///
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
    // in MiB
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
    pub limit: Option<Limit>,
    pub capacity: Vec<String>,
    pub capacity_drop: Vec<String>,
    pub with_profile: Option<PathBuf>,
    // relative path based on `.local/tulip/image`
    pub has_x: bool,
    pub env: Vec<EnvPair>,
    pub env_remove: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub systemd_nspawn: NSpawnConfig,
    pub firejail: FirejailConfig,
    pub notification: String,
    pub max_grade: usize,
    pub stdin: Option<PathBuf>,
    pub extra_bind: Vec<Binding>,
    pub extra_bind_ro: Vec<Binding>,
    pub extra_overlay: Vec<Binding>,
    pub extra_overlay_ro: Vec<Binding>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct StudentConfig {
    pub student_id: String,
    pub build_shell: PathBuf,
    pub run_shell: PathBuf,
    pub notification: String,
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