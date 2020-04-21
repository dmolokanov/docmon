use std::path::Path;

use config::{ConfigError, File};
use serde::Deserialize;

use crate::{client::ClientConfig, publish::PublisherConfig};

#[derive(Debug, Deserialize)]
pub struct Config {
    client: ClientConfig,
    publisher: PublisherConfig,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
        let mut config = config::Config::new();
        config.merge(File::from(path.as_ref()))?;

        config.try_into()
    }

    pub fn into_parts(self) -> (ClientConfig, PublisherConfig) {
        (self.client, self.publisher)
    }
}
