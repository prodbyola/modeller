use modeller::prelude::*;

#[derive(Modeller)]
pub struct TestModel {
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
