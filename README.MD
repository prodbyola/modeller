# MODELLER
Define database schema, create migrations and run migration queries all from Rust structs.

Example:
```rust
// defining models
define_models! {
    struct TestModel {
        id: u64,
        country: Option<String>,

        #[modeller(name=user_location, default=Lagos, unique)]
        state: u32
    },
    #[table_name = "custom_table_name"]
    struct AnotherModel {
        #[modeller(serial)]
        id: u64,

        #[modeller(unique, length=12)]
        username: String,

        #[modeller(default=18)]
        age: Option<u32>,

        #[modeller(type=NULLABLE TEXT)]
        bio: String
    },
}

// result migration query for creating tables:
// CREATE TABLE test_model (
// 	id BIGINT NOT NULL,
// 	country VARCHAR,
// 	user_location INTEGER NOT NULL UNIQUE DEFAULT Lagos
// );

// CREATE TABLE custom_table_name (
// 	id INTEGER PRIMARY KEY AUTOINCREMENT,
// 	username VARCHAR(12) NOT NULL UNIQUE,
// 	age INTEGER  DEFAULT 18,
// 	bio TEXT
// );

// You can still access your models anywhere depending
// on visibility
impl TestModel{}
impl AnotherModel{}
```

Please see flow description for [how it works](#FLOW.MD).
