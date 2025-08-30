use scraper::Selector;
use serde::{Deserialize, Deserializer};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub database: PathBuf,
    pub page: Vec<Page>,
    pub pushover: Option<Pushover>,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    pub label: String,
    pub url: String,

    #[serde(deserialize_with = "deserialize_selector")]
    pub selector: Selector,
}

#[derive(Debug, Deserialize)]
pub struct Pushover {
    pub token: String,
    pub user: String,
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

fn deserialize_selector<'de, D>(
    deserializer: D,
) -> Result<Selector, D::Error>
where
    D: Deserializer<'de>,
{
    let selector_str = String::deserialize(deserializer)?;

    Selector::parse(&selector_str).map_err(serde::de::Error::custom)
}
