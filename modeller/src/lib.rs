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

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[tokio::test]
    async fn test_modeller() -> OpResult<()> {
        // define one or more models in a specific module.
        #[derive(Modeller)]
        struct TestModel {
            #[modeller(serial)]
            id: u64,
            country: Option<String>,

            #[modeller(name = "user_location", default = "Lagos", unique)]
            state: String,
            // #[modeller(default=CURRENT_TIMESTAMP)]
            // created_at: Datetime
        }

        #[derive(Modeller)]
        #[modeller(table_name = "custom_table_name")]
        pub struct AnotherModel {
            #[modeller(serial)]
            id: u64,

            #[modeller(unique, length = 12)]
            username: String,

            #[modeller(default = "18")]
            age: Option<u32>,

            #[modeller(type = "NULLABLE TEXT")]
            bio: u32,
        }

        #[derive(Modeller)]
        #[modeller(unique_together(name, puk))]
        pub struct Product {
            id: u64,
            name: String,
            puk: String,
        }

        // write streams for each model
        let config = Config::default();
        TestModel::write_stream(&config).await?;
        AnotherModel::write_stream(&config).await?;
        Product::write_stream(&config).await?;

        // in your main, lib or mod
        let runner = run_modeller(&config).await;
        assert!(runner.is_ok());

        Ok(())
    }
}
