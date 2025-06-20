use definitions::bincode::{self, config};
use rbs::Value;
use std::path::PathBuf;

use crate::{OpResult, config::Config, errors::Error, generate_migration_filename, open_file};
use definitions::{backend_type::BackendType, model::ModelDefinition};
use rbatis::RBatis;
use rbdc_mysql::MysqlDriver;
use rbdc_pg::PgDriver;
use rbdc_sqlite::SqliteDriver;
use tokio::io::AsyncWriteExt;

pub struct ModellerExec {
    bt: BackendType,
    pool: RBatis,
    config: Config,
}

impl ModellerExec {
    /// run Modeller instance
    pub async fn run(&self) -> OpResult<()> {
        self.connect().await?;
        self.create_migrations_table().await?;

        // create metadata file if it does not exist
        let mf = self.metadata_filename();
        if !mf.is_file() {
            self.creation_metadata_file().await?;
        }

        // load raw data
        let stream = self.load_stream().await?;
        let metadata = self.load_metadata().await?;

        if metadata.is_empty() {
            self.init_query(&stream).await?;
        } else if metadata != stream {
            self.generate_migrations(&stream, &metadata).await?;
        } else {
            println!("modeller: no model changes detected.")
        }

        // run migrations and update metadata
        self.run_pending_migrations().await?;
        self.update_metadata(&stream).await?;

        // remove raw stream
        self.remove_stream().await?;

        Ok(())
    }

    /// create migrations table if it does not exist
    async fn create_migrations_table(&self) -> OpResult<()> {
        let table_name = self.config.migrations_table();
        let query = format!(
            "
            CREATE TABLE IF NOT EXISTS {table_name} (
                filename VARCHAR(200) NOT NULL UNIQUE
            );"
        );

        self.pool.exec(&query, vec![]).await?;
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

        let rb = &self.pool;
        let url = &self.config.db_url();

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
        let content = create_sqls.join("\n");

        file.write_all(content.as_bytes()).await?;

        Ok(())
    }

    pub fn new(config: &Config) -> Self {
        // let config = config.unwrap_or_default();
        let db_url = config.db_url();

        let bt = db_url.into();
        let pool = RBatis::new();

        Self {
            pool,
            bt,
            config: config.clone(),
        }
    }

    /// Get the name of a file or folder within migrations folder
    fn get_migration_child(&self, child_name: &str) -> PathBuf {
        let dir = &self.config.migrations_dir();
        dir.join(child_name)
    }

    fn metadata_filename(&self) -> PathBuf {
        let path = &self.config.metadata_path();
        path.to_path_buf()
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
        let table_name = self.config.migrations_table();
        let done_migs = self
            .pool
            .query(&format!("SELECT filename from {table_name}"), vec![])
            .await?;

        let results: Vec<String> = done_migs
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter_map(|v| v.as_map().map(|m| m.get(&Value::from("filename")).into()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// get list of all migration files from migrations directory
    async fn migration_files(&self) -> OpResult<Vec<PathBuf>> {
        let dir = &self.config.migrations_dir();

        let mut entries = tokio::fs::read_dir(dir).await?;
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

                self.pool.exec(&sql, vec![]).await?;

                // update migration status
                let table_name = self.config.migrations_table();
                let filename = mig.to_str().unwrap_or("");
                let insert_query = format!("INSERT INTO {table_name} (filename) VALUES(?)");
                self.pool.exec(&insert_query, vec![filename.into()]).await?;
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
        let pm = decode_raw(metadata)?; // previous models
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
    pub async fn write_stream(stream: &mut Vec<u8>, config: &Config) -> OpResult<()> {
        let mig_dir = config.migrations_dir();
        let stream_path = config.stream_path();

        if !mig_dir.is_dir() {
            tokio::fs::create_dir_all(mig_dir).await?;
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&stream_path)
            .await?;

        let stream_size = stream.len() as u32;
        let mut stream_size_block = stream_size.to_ne_bytes().to_vec();

        let mut total: u8 = 1;
        let mut blocks = Vec::new();
        let mut models = Vec::new();
        let mut content = Vec::new();

        let fc = tokio::fs::read(stream_path).await?;

        // check if stream file already exists
        if let Some(count) = fc.first() {
            let total_blocks = (*count as usize) * 4;
            let mut iter = fc.iter();
            iter.next(); // take out first byte (count)

            blocks = iter.clone().take(total_blocks).copied().collect();
            total = count + 1;

            models = iter.skip(total_blocks).copied().collect();
        }

        blocks.append(&mut stream_size_block);
        models.append(stream);

        content.push(total);
        content.append(&mut blocks);
        content.append(&mut models);

        file.write_all(&content).await?;

        Ok(())
    }

    async fn load_stream(&self) -> OpResult<Vec<u8>> {
        let mf = &self.config.stream_path();
        let content = tokio::fs::read(&mf).await?;

        Ok(content)
    }

    async fn remove_stream(&self) -> OpResult<()> {
        let mp = &self.config.stream_path();
        tokio::fs::remove_file(mp).await?;
        Ok(())
    }
}

// fn migrations_dir() -> PathBuf {
//     let dir = std::env::var(MIG_DIR_KEY).unwrap_or(DEFAULT_MIG_DIR.to_string());
//     let p = self.;
//     p.join(&dir)
// }

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
