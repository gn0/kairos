use scraper::Selector;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Page {
    pub label: String,
    pub url: String,

    #[serde(deserialize_with = "deserialize_selector")]
    pub selector: Selector,
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
