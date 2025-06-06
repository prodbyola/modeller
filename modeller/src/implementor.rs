use std::path::Path;

use crate::{
    DB_URL_KEY, DEFAULT_DB, DEFAULT_MIG_DIR, MIG_DIR_KEY, MIG_TABLE_NAME, errors::OpResult,
    generate_migration_filename, metadata_filename, open_file,
};
use definitions::{backend_type::BackendType, model::ModelDefinition, serde_json};
use rbatis::RBatis;
use rbdc_mysql::MysqlDriver;
use rbdc_pg::PgDriver;
use rbdc_sqlite::SqliteDriver;
use tokio::io::AsyncWriteExt;

pub struct Modeller<'a> {
    bt: BackendType,
    db_url: String,
    db_pool: RBatis,
    migrations_dir: String,
    raw: &'a [String],
}

impl<'a> Modeller<'a> {
    pub fn new(raw_models: &'a [String]) -> Self {
        let db_url = std::env::var(DB_URL_KEY).unwrap_or(DEFAULT_DB.to_string());
        let migrations_dir = std::env::var(MIG_DIR_KEY).unwrap_or(DEFAULT_MIG_DIR.to_string());
        let bt = db_url.as_str().into();
        let db_pool = RBatis::new();

        Self {
            db_pool,
            db_url,
            migrations_dir,
            bt,
            raw: raw_models,
        }
    }

    /// run Modeller instance
    pub async fn run(&self) -> OpResult<()> {
        let dir = Path::new(&self.migrations_dir);
        let dir_exists = dir.exists() && dir.is_dir();

        if !dir_exists {
            self.init().await?;
            self.run_first_migration().await?;
        }

        Ok(())
    }

    /// initializes modeller.
    /// - attempts to connect to the database
    /// - create database "migrations" table if it doesn't exist
    /// - create "migrations" directory and metadata file if they don't exist.
    async fn init(&self) -> OpResult<()> {
        // perform init
        self.connect().await?;
        self.create_migrations_table().await?;
        self.create_migrations_folder().await?;

        Ok(())
    }

    async fn create_migrations_table(&self) -> OpResult<()> {
        let query = format!(
            "
            DROP TABLE IF EXISTS {MIG_TABLE_NAME};

            CREATE TABLE IF NOT EXISTS {MIG_TABLE_NAME} (
                filename VARCHAR(200) NOT NULL UNIQUE,
                run_status BOOLEAN DEFAULT false
            );"
        );

        self.db_pool.exec(&query, vec![]).await?;
        Ok(())
    }

    /// create migrations dir and all initial files. Caller
    /// should verify if migrations dir exists when required.
    async fn create_migrations_folder(&self) -> OpResult<()> {
        let dir = Path::new(&self.migrations_dir);
        tokio::fs::create_dir_all(dir).await?;
        tokio::fs::File::create(&metadata_filename()).await?;

        Ok(())
    }

    async fn connect(&self) -> OpResult<()> {
        use BackendType::*;
        let rb = &self.db_pool;
        let url = &self.db_url;

        match self.bt {
            Sqlite => rb.link(SqliteDriver {}, url).await?,
            MySql => rb.link(MysqlDriver {}, url).await?,
            Postgres => rb.link(PgDriver {}, url).await?,
        }

        Ok(())
    }

    async fn run_first_migration(&self) -> OpResult<()> {
        let models = self.models();
        let create_sqls: Vec<String> = models
            .iter()
            .map(|model| model.create_table_sql(&self.bt))
            .collect();

        let mig_filename = generate_migration_filename(&self.migrations_dir);
        let mig_content = create_sqls.join("\n\n");

        // create migration file
        let mut file = open_file(&mig_filename).await?;
        file.write_all(mig_content.as_bytes()).await?;

        // write metadata
        let raw_content = serde_json::to_string(&self.raw)?;
        let mut file = open_file(&metadata_filename()).await?;
        file.write_all(raw_content.as_bytes()).await?;

        // run the migration
        self.db_pool.exec(&mig_content, vec![]).await?;

        // update migration status
        let insert_query =
            format!("INSERT INTO {MIG_TABLE_NAME} (filename, run_status) VALUES(?, true)");
        self.db_pool
            .exec(&insert_query, vec![mig_filename.into()])
            .await?;

        Ok(())
    }

    fn models(&self) -> Vec<ModelDefinition> {
        self.raw
            .iter()
            .map(|rm| {
                serde_json::from_str(rm).expect("unable to parse model definition from raw_str")
            })
            .collect()
    }
}
