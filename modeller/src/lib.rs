use std::path::PathBuf;
use crate::{config::Config, exec::ModellerExec};

mod config;
mod errors;
mod exec;

pub mod prelude;

#[cfg(test)]
#[allow(dead_code)]
mod tests;

pub use errors::Error;

pub type OpResult<T> = Result<T, errors::Error>;

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
