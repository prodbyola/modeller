use std::path::PathBuf;

use chrono::Utc;

use crate::{config::Config, implementor::ModellerExec};

mod config;
mod errors;
mod implementor;

pub mod prelude;

pub use errors::Error;

pub type OpResult<T> = Result<T, errors::Error>;

fn generate_migration_filename() -> String {
    let now = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    format!("migration_{now}.sql")
}

async fn open_file(path: &PathBuf) -> OpResult<tokio::fs::File> {
    let f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .write(true)
        .open(path)
        .await?;

    Ok(f)
}

pub async fn run_modeller(config: &Config) -> Result<(), errors::Error> {
    let modeller = ModellerExec::new(config);
    modeller.run().await
}