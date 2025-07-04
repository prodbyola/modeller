use std::sync::Mutex;

use modeller::prelude::*;

use models::*;
use once_cell::sync::Lazy;

#[allow(dead_code)]
mod models;

static LAZY_CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| Mutex::new(Config::default()));

#[tokio::test]
async fn test_modeller() -> OpResult<()> {
    // write streams for each model
    let config = LAZY_CONFIG.lock().unwrap();
    TestModel::write_stream(&config).await?;
    AnotherModel::write_stream(&config).await?;
    Product::write_stream(&config).await?;

    // in your main, lib or mod
    let runner = run_modeller(&config).await;
    if let Err(err) = &runner {
        eprintln!("test failed: {err}")
    }

    assert!(runner.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_modeller_version2() -> OpResult<()> {
    // write streams for each model
    let config = LAZY_CONFIG.lock().unwrap();
    TestModel::write_stream(&config).await?;
    AnotherModel::write_stream(&config).await?;
    Product::write_stream(&config).await?;

    // in your main, lib or mod
    let runner = run_modeller(&config).await;
    if let Err(err) = &runner {
        eprintln!("test failed: {err}")
    }

    assert!(runner.is_ok());

    Ok(())
}