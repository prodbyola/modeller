use modeller::prelude::*;

use models::*;
use serial_test::serial;

#[allow(dead_code)]
mod models;

#[tokio::test]
#[serial]
async fn test_modeller() -> OpResult<()> {
    // write streams for each model
    let mut config = Config::default();
    TestModel::write_stream(&mut config);
    AnotherModel::write_stream(&mut config);
    Product::write_stream(&mut config);

    // in your main, lib or mod
    let runner = run_modeller(&config).await;
    if let Err(err) = &runner {
        eprintln!("test failed: {err}")
    }

    assert!(runner.is_ok());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_modeller_version2() -> OpResult<()> {
    // write streams for each model
    let mut config = Config::default();
    TestModel::write_stream(&mut config);
    AnotherModel::write_stream(&mut config);
    Product::write_stream(&mut config);

    // in your main, lib or mod
    let runner = run_modeller(&config).await;
    if let Err(err) = &runner {
        eprintln!("test failed: {err}")
    }

    assert!(runner.is_ok());

    Ok(())
}