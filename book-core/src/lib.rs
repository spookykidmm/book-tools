pub mod types;
pub mod scanner;
pub mod cleaner;
pub mod merger;
pub mod metadata;
pub mod organizer;
pub mod config;
pub mod hashing;

use anyhow::Result;
use std::path::PathBuf;

pub trait Tool {
    fn run(&self) -> Result<()>;
    fn dry_run(&self) -> Result<()>;
    fn validate(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Context {
    pub config: config::Config,
    pub manifest: types::Manifest,
    pub output: PathBuf,
}
