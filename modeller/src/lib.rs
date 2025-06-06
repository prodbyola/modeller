use chrono::Utc;

use crate::errors::OpResult;

pub mod errors;
pub mod implementor;

const DB_URL_KEY: &str = "MODELLER_DATABASE_URL";
const MIG_DIR_KEY: &str = "MODELLER_MIGRATIONS_DIR";
const DEFAULT_DB: &str = "sqlite://db.sqlite";
const DEFAULT_MIG_DIR: &str = "migrations";
const MIG_TABLE_NAME: &str = "mmm_migrations";

fn metadata_filename() -> String {
    format!("{DEFAULT_MIG_DIR}/metadata")
}

fn generate_migration_filename(mig_dir: &str) -> String {
    let now = Utc::now().to_string();
    format!("{}/migration_{now}.sql", mig_dir)
}

async fn open_file(path: &str) -> OpResult<tokio::fs::File> {
    let f = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)
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
        use definitions::core::DefinitionStream;
        use modeller_parser::parse_models;
        use crate::implementor::Modeller;

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

        pub fn modeller_stream() -> DefinitionStream {
            get_raw_definitions()
        }

        pub fn get_modeller(models: &[String]) -> Modeller {
            Modeller::new(models)
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::errors::OpResult;

    #[tokio::test]
    async fn test_modeller() -> OpResult<()> {
        define_models! {
            #[table_name = "custom_table_name"]
            struct TestModel {
                id: u64,
                country: Option<String>,

                #[modeller(name=user_location, default=Lagos, unique)]
                state: u32,

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
                bio: String
            }
        }

        let stream = modeller_stream();
        let modeller = get_modeller(stream.models());

        modeller.run().await?;

        Ok(())
    }
}
