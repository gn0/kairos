use serde::Deserialize;
use std::path::PathBuf;

use crate::page::Page;
use crate::pushover::Pushover;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: PathBuf,
    pub page: Vec<Page>,
    pub pushover: Option<Pushover>,
}

impl Config {
    /// Loads the configuration from the specified location.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    ///
    /// - the configuration file doesn't exist or
    /// - the configuration file contains a parse error.
    pub fn load(path: &str) -> Result<Self, String> {
        let config: Config = toml::from_str(
            &std::fs::read_to_string(path)
                .map_err(|error| format!("{path:?}: {error}"))?,
        )
        .map_err(|error| error.message().to_string())?;

        Ok(config)
    }
}
