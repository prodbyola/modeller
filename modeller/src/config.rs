use std::path::PathBuf;

#[derive(Clone)]
pub struct Config {
    db_url: String,
    metadata_path: PathBuf,
    streams: Vec<u8>,
}

impl Config {
    pub fn db_url(&self) -> &str {
        &self.db_url
    }

    pub fn metadata_path(&self) -> &PathBuf {
        &self.metadata_path
    }

    pub fn streams(&self) -> &[u8] {
        &self.streams
    }

    pub fn write_streams(&mut self, streams: Vec<u8>) {
        self.streams = streams;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_url: "sqlite://db.sqlite".to_string(),
            metadata_path: "modeller_data".into(),
            streams: vec![],
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

    pub fn metadata_path<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.config.metadata_path = dir.into();
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
