use definitions::bincode::{self, config};
use std::path::{Path, PathBuf};

use crate::{
    DB_URL_KEY, DEFAULT_DB, DEFAULT_MIG_DIR, METADATA_FILENAME, MIG_DIR_KEY, MIG_TABLE_NAME,
    errors::{Error, OpResult},
    generate_migration_filename, open_file,
};
use definitions::{backend_type::BackendType, model::ModelDefinition};
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
    raw: &'a [u8],
}

impl<'a> Modeller<'a> {
    /// run Modeller instance
    pub async fn run(&self) -> OpResult<()> {
        let dir = Path::new(&self.migrations_dir);
        let dir_exists = dir.exists() && dir.is_dir();

        if !dir_exists {
            self.init().await?;
            self.run_first_migration().await?;
        } else {
            let metadata = self.load_metadata().await?;
            let raw = self.raw;

            if &metadata == raw {
                println!("modeller: no changes detected!")
            }
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
        let mig_dir = self.migrations_path();
        let mf = self.metadata_filename()?;

        tokio::fs::create_dir_all(mig_dir).await?;
        tokio::fs::File::create(&mf).await?;

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

        // create migration file
        let mut mig_filename = generate_migration_filename();
        mig_filename = self.build_mig_path(&mig_filename)?;

        let mut file = open_file(&mig_filename).await?;
        let content = create_sqls.join("\n\n");

        file.write_all(content.as_bytes()).await?;

        // write metadata
        let mf = self.metadata_filename()?;
        let mut file = open_file(&mf).await?;
        file.write_all(&self.raw).await?;

        // run the migration
        self.db_pool.exec(&content, vec![]).await?;

        // update migration status
        let insert_query =
            format!("INSERT INTO {MIG_TABLE_NAME} (filename, run_status) VALUES(?, true)");
        self.db_pool
            .exec(&insert_query, vec![mig_filename.into()])
            .await?;

        Ok(())
    }

    fn models(&self) -> Vec<ModelDefinition> {
        let config = config::standard();
        match bincode::decode_from_slice(&self.raw, config) {
            Ok((encoded, _)) => encoded,
            Err(_) => vec![],
        }
    }

    pub fn new(raw: &'a [u8]) -> Self {
        let db_url = std::env::var(DB_URL_KEY).unwrap_or(DEFAULT_DB.to_string());
        let migrations_dir = std::env::var(MIG_DIR_KEY).unwrap_or(DEFAULT_MIG_DIR.to_string());
        let bt = db_url.as_str().into();
        let db_pool = RBatis::new();

        Self {
            db_pool,
            db_url,
            migrations_dir,
            bt,
            raw,
        }
    }

    fn migrations_path(&self) -> PathBuf {
        let path = PathBuf::new();
        path.join(&self.migrations_dir)
    }

    fn build_mig_path(&self, child_name: &str) -> OpResult<String> {
        let path = self.migrations_path().join(child_name);

        let path_str = path.to_str().ok_or(Error::ParseError(
            "unable to parse migration file".to_string(),
        ))?;

        Ok(path_str.to_string())
    }

    fn metadata_filename(&self) -> OpResult<String> {
        self.build_mig_path(&METADATA_FILENAME)
    }

    async fn load_metadata(&self) -> OpResult<Vec<u8>> {
        let mf = self.migrations_path().join(&METADATA_FILENAME);
        if mf.exists() {
            let metadata = tokio::fs::read(&mf).await?;
            Ok(metadata)
        } else {
            Err(Error::InternalError("missing metadata file. you might need to delete your migrations folder or specify a different migration directory.".to_string()))
        }
    }
}
