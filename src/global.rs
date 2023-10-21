use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

use lazy_static::lazy_static;
use octocrab::Octocrab;

lazy_static! {
    pub static ref RELEASE_DIR_PATH: PathBuf = env::current_dir().unwrap().join(".renote");
}

pub static GITHUB_CLIENT: OnceLock<Octocrab> = OnceLock::new();
