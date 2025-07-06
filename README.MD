# MODELLER

Define database schema, create migrations and run migration queries all from Rust structs. Modeller currently support Sqlite, Mysql and Postgres backends.

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

#### Query Customization
###### Table Name
Modeller generates table name from struct identifier by default:
```rust
#[derive(Modeller)]
struct MockTable {
    pub id: u64,
}

// CREATE TABLE mock_table (id INTEGER NOT NULL);
```

You can customize your table name by specifying `table_name` parameter in `modeller` attribute:
```rust
#[derive(Modeller)]
#[modeller(table_name = "custom_table_name")]
struct MockTable {
    pub id: u64,
}

// CREATE TABLE custom_table_name (id INTEGER NOT NULL);
```

###### Unique Together Columns
```rust
#[derive(Modeller)]
#[modeller(unique_together(user_id, project_id))]
struct MockTable {
    pub user_id: u64,
    pub project_id: u64,
}

// CREATE TABLE mock_table (
//  ...,
//  UNIQUE (user_id, project_id)
// );
```
