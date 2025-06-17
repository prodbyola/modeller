use definitions::bincode::{self, config};
use rbs::Value;
use std::path::PathBuf;

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

pub struct Modeller {
    bt: BackendType,
    db_url: String,
    db_pool: RBatis,
}

impl Modeller {
    /// run Modeller instance
    pub async fn run(&self) -> OpResult<()> {
        self.connect().await?;

        let stream = Self::load_stream().await?;
        let metadata = self.load_metadata().await?;

        let mf = self.metadata_filename();
        if !mf.is_file() {
            self.init().await?;
            self.init_query(&metadata).await?;
        } else {
            self.generate_migrations(&stream).await?;
        }

        self.run_pending_migrations().await?;
        self.update_metadata(&stream).await?;

        Ok(())
    }

    /// initializes modeller.
    /// - create database "migrations" table if it doesn't exist
    /// - create metadata file.
    async fn init(&self) -> OpResult<()> {
        // perform init
        self.create_migrations_table().await?;
        self.creation_metadata_file().await?;

        Ok(())
    }

    async fn create_migrations_table(&self) -> OpResult<()> {
        let query = format!(
            "
            DROP TABLE IF EXISTS {MIG_TABLE_NAME};

            CREATE TABLE IF NOT EXISTS {MIG_TABLE_NAME} (
                filename VARCHAR(200) NOT NULL UNIQUE
            );"
        );

        self.db_pool.exec(&query, vec![]).await?;
        Ok(())
    }

    /// create metadata file.
    async fn creation_metadata_file(&self) -> OpResult<()> {
        let mf = self.metadata_filename();
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

    /// Generate initial query for all models. Initial query
    /// is usually SQL CREATE statements for all available models.
    ///
    /// Once the query is generated, we write it to our first
    /// migration file.
    async fn init_query(&self, metadata: &[u8]) -> OpResult<()> {
        // generate initial query for all models
        let models = decode_raw(metadata)?;
        let create_sqls: Vec<String> = models
            .iter()
            .map(|model| model.sql_create_table(&self.bt))
            .collect();

        // wite the query to first migrations file
        let filename = generate_migration_filename();
        let path = self.build_migration_path(&filename)?;

        let mut file = open_file(&path).await?;
        let content = create_sqls.join("\n\n");

        file.write_all(content.as_bytes()).await?;

        Ok(())
    }

    pub fn new() -> Self {
        let db_url = std::env::var(DB_URL_KEY).unwrap_or(DEFAULT_DB.to_string());
        let bt = db_url.as_str().into();
        let db_pool = RBatis::new();

        Self {
            db_pool,
            db_url,
            bt,
        }
    }

    fn build_migration_path(&self, child_name: &str) -> OpResult<PathBuf> {
        let path = migrations_dir().join(child_name);

        let path_str = path.to_str().ok_or(Error::ParseError(
            "unable to parse migration file".to_string(),
        ))?;

        Ok(PathBuf::new().join(path_str))
    }

    fn metadata_filename(&self) -> PathBuf {
        let md = migrations_dir();
        md.join(METADATA_FILENAME)
    }

    async fn load_metadata(&self) -> OpResult<Vec<u8>> {
        let mf = self.metadata_filename();
        if mf.exists() {
            let metadata = tokio::fs::read(&mf).await?;
            Ok(metadata)
        } else {
            Err(Error::InternalError("missing metadata file. you might need to delete your migrations folder or specify a different migration directory.".to_string()))
        }
    }

    /// get list previously ran migrations from database
    async fn previous_migrations(&self) -> OpResult<Vec<String>> {
        let done_migs = self
            .db_pool
            .query(&format!("SELECT filename from {MIG_TABLE_NAME}"), vec![])
            .await?;

        let results: Vec<String> = done_migs
            .as_array()
            .map(|rows| {
                rows.iter()
                    .map(|v| v.as_map().map(|m| m.get(&Value::from("filename")).into()))
                    .flatten()
                    .collect()
            })
            .unwrap_or(vec![]);

        Ok(results)
    }

    /// get list of all migration files from migrations directory
    async fn migration_files(&self) -> OpResult<Vec<PathBuf>> {
        let dir = migrations_dir();

        let mut entries = tokio::fs::read_dir(&dir).await?;
        let mut paths = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            paths.push(entry.path());
        }

        Ok(paths)
    }

    async fn run_pending_migrations(&self) -> OpResult<()> {
        let pvs = self.previous_migrations().await?;
        let mfs = self.migration_files().await?;
        let metafile = self.metadata_filename();

        let new_migrations: Vec<&PathBuf> = mfs
            .iter()
            .filter(|filepath| {
                if filepath == &&metafile {
                    return false;
                }

                let exists = pvs.iter().find(|pv| {
                    filepath
                        .to_str()
                        .map(|path| path == pv.as_str())
                        .unwrap_or_default()
                });
                exists.is_none()
            })
            .collect();

        if !new_migrations.is_empty() {
            for mig in new_migrations {
                let content = tokio::fs::read(mig).await?;
                let sql = String::from_utf8(content).map_err(|err| {
                    Error::InternalError(format!("error parsing migration content {mig:?}: {err}"))
                })?;

                // run the migration
                self.db_pool.exec(&sql, vec![]).await?;

                // update migration status
                let filename = mig.to_str().unwrap_or("");
                let insert_query = format!("INSERT INTO {MIG_TABLE_NAME} (filename) VALUES(?)");
                self.db_pool
                    .exec(&insert_query, vec![filename.into()])
                    .await?;
            }
        }

        Ok(())
    }

    async fn update_metadata(&self, stream: &[u8]) -> OpResult<()> {
        // write metadata
        let mf = self.metadata_filename();
        let mut file = open_file(&mf).await?;
        file.write_all(stream).await?;

        Ok(())
    }

    /// Generate migration for changed models if any.
    async fn generate_migrations(&self, stream: &[u8]) -> OpResult<()> {
        let metadata = self.load_metadata().await?;
        // let raw = self.raw;

        if &metadata != stream {
            let pm = decode_raw(&metadata)?; // previous models
            let cm = decode_raw(stream)?; // current models

            let mut queries = Vec::with_capacity(cm.len());
            let bt = &self.bt;

            for model in cm {
                // check if model already exists
                let exists = pm.iter().find(|p| model.name() == p.name());
                match exists {
                    Some(prev) => {
                        if let Some(q) = model.sql_alter_table(prev, bt) {
                            queries.push(q);
                        }
                    }
                    None => {
                        let q = model.sql_create_table(bt);
                        queries.push(q);
                    }
                }
            }
            if !queries.is_empty() {
                // wite query to migration file
                let filename = generate_migration_filename();
                let file = self.build_migration_path(&filename)?;

                let mut file = open_file(&file).await?;
                let content = queries.join("\n\n");

                file.write_all(content.as_bytes()).await?;
            }
        } else {
            println!("modeller: no changes detected!")
        }

        Ok(())
    }

    /// Write generated metadata streams to metadata file
    pub async fn write_stream(stream: &mut Vec<u8>) -> OpResult<()> {
        let mp = migrations_dir();
        let mf = mp.join("stream");

        if !mp.is_dir() {
            tokio::fs::create_dir_all(mp).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&mf)
            .await?;

        let mut fc = tokio::fs::read(mf).await?;

        let content = if !fc.is_empty() {
            fc.append(stream);
            &fc
        } else {
            stream
        };

        file.write(&content).await?;

        Ok(())
    }

    async fn load_stream() -> OpResult<Vec<u8>> {
        let mp = migrations_dir();
        let mf = mp.join("stream");

        let content = tokio::fs::read(&mf).await?;

        Ok(content)
    }
}

fn migrations_dir() -> PathBuf {
    let dir = std::env::var(MIG_DIR_KEY).unwrap_or(DEFAULT_MIG_DIR.to_string());
    let p = PathBuf::new();
    p.join(&dir)
}

fn decode_raw(raw: &[u8]) -> OpResult<Vec<ModelDefinition>> {
    let config = config::standard();
    let m: (Vec<ModelDefinition>, usize) = bincode::decode_from_slice(raw, config)
        .map_err(|err| Error::InternalError(err.to_string()))?;

    Ok(m.0)
}
