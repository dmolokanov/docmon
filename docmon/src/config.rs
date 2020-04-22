use std::path::Path;

use config::{ConfigError, Environment, File};
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
        config.merge(Environment::with_prefix("DOCMON_").separator("__"))?;

        config.try_into()
    }

    pub fn into_parts(self) -> (ClientConfig, PublisherConfig) {
        (self.client, self.publisher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_converts_toml_config() {
        let content = r#"inner_field.field_bar = "value""#.to_string();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, content).unwrap();

        let mut config = config::Config::new();
        config.merge(File::from(path)).unwrap();

        let c: TestConfig = config.try_into().unwrap();
        assert_eq!(c.inner_field.field_bar, "value")
    }

    #[test]
    fn it_merges_config_with_env() {
        let content = r#"inner_field.field_bar = "value""#.to_string();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, content).unwrap();

        std::env::set_var("TEST__INNER_FIELD__FIELD_BAR", "new-value");

        let mut config = config::Config::new();
        config.merge(File::from(path)).unwrap();
        config
            .merge(Environment::with_prefix("TEST_").separator("__"))
            .unwrap();

        let c: TestConfig = config.try_into().unwrap();
        assert_eq!(c.inner_field.field_bar, "new-value")
    }

    #[derive(Debug, Deserialize)]
    struct TestConfig {
        inner_field: InnerTestConfig,
    }

    #[derive(Debug, Deserialize)]
    struct InnerTestConfig {
        field_bar: String,
    }
}
