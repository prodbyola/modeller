use crate::prelude::*;

mod models;
use models::*;

#[tokio::test]
async fn test_modeller() -> OpResult<()> {
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
