use definitions::bincode::{self, config};
use rbs::Value;
use std::path::PathBuf;

use crate::{
    DB_URL_KEY, DEFAULT_DB, DEFAULT_MIG_DIR, METADATA_FILENAME, MIG_DIR_KEY, MIG_TABLE_NAME,
    OpResult, errors::Error, generate_migration_filename, open_file,
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
        // connect to database
        self.connect().await?;
        self.create_migrations_table().await?;

        // create metadata file if it does not exist
        let mf = self.metadata_filename();
        if !mf.is_file() {
            self.creation_metadata_file().await?;
        }

        // load raw data
        let stream = Self::load_stream().await?;
        let metadata = self.load_metadata().await?;

        if metadata.is_empty() {
            self.init_query(&stream).await?;
        } else {
            if metadata != stream {
                self.generate_migrations(&stream, &metadata).await?;
            } else {
                println!("modeller: no model changes detected.")
            }
        }

        // run migrations and update metadata
        self.run_pending_migrations().await?;
        self.update_metadata(&stream).await?;

        // remove raw stream
        Self::remove_stream().await?;

        Ok(())
    }

    /// create migrations table if it does not exist
    async fn create_migrations_table(&self) -> OpResult<()> {
        let query = format!(
            "
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
    async fn init_query(&self, stream: &[u8]) -> OpResult<()> {
        // generate initial query for all models
        let models = decode_raw(stream)?;
        let create_sqls: Vec<String> = models
            .iter()
            .map(|model| model.sql_create_table(&self.bt))
            .collect();

        // wite the query to first migrations file
        let filename = generate_migration_filename();
        let path = self.get_migration_child(&filename);

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

    /// Get the name of a file or folder within migrations folder
    fn get_migration_child(&self, child_name: &str) -> PathBuf {
        migrations_dir().join(child_name)
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
        let streamfile = self.get_migration_child("stream");

        let new_migrations: Vec<&PathBuf> = mfs
            .iter()
            .filter(|filepath| {
                // exclude other files
                if [&metafile, &streamfile].contains(filepath) {
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
                // run the migration
                let content = tokio::fs::read(mig).await?;
                let sql = String::from_utf8(content).map_err(|err| {
                    Error::InternalError(format!("error parsing migration content {mig:?}: {err}"))
                })?;

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
    async fn generate_migrations(&self, stream: &[u8], metadata: &[u8]) -> OpResult<()> {
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
            let file = self.get_migration_child(&filename);

            let mut file = open_file(&file).await?;
            let content = queries.join("\n\n");

            file.write_all(content.as_bytes()).await?;
        }

        Ok(())
    }

    /// Write generated metadata streams to metadata file
    pub async fn write_stream(stream: &mut Vec<u8>) -> OpResult<()> {
        let md = migrations_dir();
        let mf = md.join("stream");

        if !md.is_dir() {
            tokio::fs::create_dir_all(md).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&mf)
            .await?;

        let stream_size = stream.len() as u32;
        let mut stream_size_block = stream_size.to_ne_bytes().to_vec();

        let mut total: u8 = 1;
        let mut blocks = Vec::new();
        let mut models = Vec::new();
        let mut content = Vec::new();

        let fc = tokio::fs::read(mf).await?;

        // check if stream file already exists
        if let Some(count) = fc.first() {
            let total_blocks = (*count as usize) * 4;
            let mut iter = fc.iter();
            iter.next(); // take out first byte (count)

            blocks = iter.clone().take(total_blocks).map(|b| b.clone()).collect();
            total = count + 1;

            models = iter.skip(total_blocks).map(|b| b.clone()).collect();
        }

        blocks.append(&mut stream_size_block);
        models.append(stream);

        content.push(total);
        content.append(&mut blocks);
        content.append(&mut models);

        file.write(&content).await?;

        Ok(())
    }

    async fn load_stream() -> OpResult<Vec<u8>> {
        let mp = migrations_dir();
        let mf = mp.join("stream");

        let content = tokio::fs::read(&mf).await?;

        Ok(content)
    }

    async fn remove_stream() -> OpResult<()> {
        let mp = migrations_dir();
        let mf = mp.join("stream");

        tokio::fs::remove_file(&mf).await?;
        Ok(())
    }
}

fn migrations_dir() -> PathBuf {
    let dir = std::env::var(MIG_DIR_KEY).unwrap_or(DEFAULT_MIG_DIR.to_string());
    let p = PathBuf::new();
    p.join(&dir)
}

fn decode_raw(raw: &[u8]) -> OpResult<Vec<ModelDefinition>> {
    let config = config::standard();

    let results = match raw.first() {
        Some(count) => {
            let mut iter = raw.iter();
            iter.next();

            let block_space = (*count as usize) * 4;
            let blocks = iter.clone().take(block_space).cloned();
            let mut block_sizes: Vec<u32> = Vec::with_capacity(*count as usize);

            for i in 0..*count {
                let offset = (4 * i) as usize;
                let window = blocks.clone().skip(offset).take(4).collect::<Vec<_>>();

                let mut arr = [0u8; 4];
                arr.copy_from_slice(&window[0..4]);

                let s = u32::from_ne_bytes(arr);
                block_sizes.push(s);
            }

            let mut models = Vec::with_capacity(block_sizes.len());
            let mut offset = 0;

            let contents = iter.clone().skip(block_space);
            for size in block_sizes {
                let content = contents
                    .clone()
                    .skip(offset)
                    .take(size as usize)
                    .cloned()
                    .collect::<Vec<_>>();

                let (model, _): (ModelDefinition, usize) =
                    bincode::decode_from_slice(&content, config)
                        .map_err(|err| Error::InternalError(err.to_string()))?;

                models.push(model);
                offset += size as usize;
            }

            models
        }
        None => vec![],
    };

    Ok(results)
}
