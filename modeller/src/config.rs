use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    db_url: String,
    migrations_dir: PathBuf,
    migrations_table: String,
    metadata_filename: String,
    stream_filename: String,
}

impl Config {
    pub fn db_url(&self) -> &str {
        &self.db_url
    }

    pub fn migrations_dir(&self) -> &PathBuf {
        &self.migrations_dir
    }

    pub fn migrations_table(&self) -> &str {
        &self.migrations_table
    }

    pub fn metadata_path(&self) -> PathBuf {
        let name = &self.metadata_filename;
        let path = &self.migrations_dir.join(name);

        path.to_path_buf()
    }

    pub fn stream_path(&self) -> PathBuf {
        let name = &self.stream_filename;
        let path = &self.migrations_dir.join(name);

        path.to_path_buf()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_url: "sqlite://db.sqlite".to_string(),
            migrations_dir: "migrations".into(),
            migrations_table: "mmm_migrations".to_string(),
            metadata_filename: "metadata".to_string(),
            stream_filename: "stream".to_string(),
        }
    }
}

pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
        }
    }

    pub fn db_url(mut self, url: &str) -> Self {
        self.config.db_url = url.to_string();
        self
    }

    pub fn migrations_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.config.migrations_dir = dir.into();
        self
    }

    pub fn migrations_table(mut self, table: &str) -> Self {
        self.config.migrations_table = table.to_string();
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}
