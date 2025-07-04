# MODELLER

Define database schema, create migrations and run migration queries all from Rust structs.

#### Example

Modeller is designed for with ease of integration in mind:

```rust
    // models.rs
    use modeller::prelude::*;

    // 1. define your model. Feel free to define as many models as possible
    #[derive(Modeller)]
    struct TestModel {
        id: u64,
    }

    // 2. Get Modeller Config. You can also use ConfigBuilder to create custom configuration
    let config = Config::default();

    // 3. write data streams for each model.
    TestModel::write_stream(&config).await?;

    // 4. run modeller instance
    run_modeller(&config).await?;

```
