use std::path::PathBuf;

use chrono::Utc;

use crate::errors::OpResult;
pub use modeller_parser;

pub mod errors;
pub mod implementor;

const DB_URL_KEY: &str = "MODELLER_DATABASE_URL";
const MIG_DIR_KEY: &str = "MODELLER_MIGRATIONS_DIR";
const DEFAULT_DB: &str = "sqlite://db.sqlite";
const DEFAULT_MIG_DIR: &str = "migrations";
const MIG_TABLE_NAME: &str = "mmm_migrations";
const METADATA_FILENAME: &str = "metadata";

fn generate_migration_filename() -> String {
    let now = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    format!("migration_{now}.sql")
}

async fn open_file(path: &PathBuf) -> OpResult<tokio::fs::File> {
    let f = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .await?;

    Ok(f)
}

#[macro_export]
macro_rules! define_models {
    (
        $(
            $(#[$meta:meta])*
            $vis:vis struct $name:ident {
                $(
                    $(#[$field_attr:meta])*
                    $field_vis:vis $field:ident : $ty:ty
                ),* $(,)?
            }
        ),*
    ) => {
        use modeller_parser::parse_models;
        use $crate::implementor::Modeller;
        use $crate::errors::Error;

        // parse the input models into a vector of strigified
        // `ModelDefinition`
        parse_models! {
            $(
                $(#[$meta])*
                $vis struct $name {
                    $(
                        $(#[$field_attr])*
                        $field_vis $field: $ty,
                    )*
                }
            ),*,
        }

        pub async fn write_stream(stream: &mut Vec<u8>) -> Result<(), Error> {
            Modeller::write_stream(stream).await?;
            Ok(())
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::errors::OpResult;

    #[tokio::test]
    async fn test_modeller() -> OpResult<()> {
        define_models! {
            struct TestModel {
                id: u64,
                country: Option<String>,

                #[modeller(name=user_location, default=Lagos, unique)]
                state: String,

                // #[modeller(default=CURRENT_TIMESTAMP)]
                // created_at: Datetime
            },
            #[table_name = "custom_table_name"]
            pub struct AnotherModel {
                #[modeller(serial)]
                id: u64,

                #[modeller(unique, length=12)]
                username: String,

                #[modeller(default=18)]
                age: Option<u32>,

                #[modeller(type=NULLABLE TEXT)]
                bio: u32
            }
        }

        let mut streams = modeller_definition_streams();
        write_stream(&mut streams).await?;

        let modeller = Modeller::new();
        modeller.run().await?;

        Ok(())
    }
}
