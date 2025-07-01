# MODELLER
Define database schema, create migrations and run migration queries all from Rust structs.

#### Example
Example:
```rust
    // models.rs
    use modeller::prelude::*;

    // define one or more models in a specific module.
    #[derive(Modeller)]
    struct TestModel {
        #[modeller(serial)]
        id: u64,
        country: Option<String>,

        #[modeller(name = "user_location", default = "Lagos", unique)]
        state: String,

        #[modeller(default=CURRENT_TIMESTAMP)]
        created_at: DateTime
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
```

Now let modeller generate and run necessary migrations for each model at build time.
```rust
// migrate.rs
use modeller::prelude::*;
use models::*;

async fn run_modeller() -> Result<(), Error> {
    // You can also use ConfigBuilder to create custom configuration
    let config = Config::default();

    // write streams for each model
    TestModel::write_stream(&config).await?;
    AnotherModel::write_stream(&config).await?;
    Product::write_stream(&config).await?;
    
    // run modeller instance
    run_modeller(&config).await?;
}
```

#### Lockable Config
If your app is trying to run multiple instances of modeller (for example, running an instance in multiple unit tests), you can prevent race conditions by creating a global lockable `Config` that you can share with different instances:

- Install [once-cell](https://docs.rs/once_cell/latest/once_cell/)
```.toml
# Cargo.toml
[dev-dependencies] # use [dependencies] if necessary
once_cell = "1.21.3" # use latest version
```

- Create and use global config
```rust
use once_cell::sync::Lazy;

static LAZY_CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| {
    // or use ConfigBuilder
    Mutex::new(Config::default())
});

#[tokio::test]
async fn runner_one() {
    let config = LAZY_CONFIG.lock().unwrap();
    ... // use config
}

#[tokio::test]
async fn runner_two() {
    let config = LAZY_CONFIG.lock().unwrap();
    ... // use config
}

```


Please see flow description for [how it works](https://github.com/prodbyola/modeller/blob/main/FLOW.MD).
