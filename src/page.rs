use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Deserializer};

use crate::request::client_with_retry;

#[derive(Debug, Deserialize, Clone)]
pub struct Page {
    pub name: String,
    pub url: String,

    #[serde(deserialize_with = "deserialize_selector")]
    pub selector: Selector,
}

fn deserialize_selector<'de, D>(
    deserializer: D,
) -> std::result::Result<Selector, D::Error>
where
    D: Deserializer<'de>,
{
    let selector_str = String::deserialize(deserializer)?;

    Selector::parse(&selector_str).map_err(serde::de::Error::custom)
}

impl Page {
    pub async fn request(&self) -> Result<Vec<Link>> {
        let body = client_with_retry()
            .get(&self.url)
            .send()
            .await?
            .text()
            .await?;
        let html = Html::parse_fragment(&body);

        Ok(html.select(&self.selector).map(Link::from).collect())
    }
}

#[derive(Debug)]
pub struct Link {
    pub href: String,
    pub text: String,
}

impl From<ElementRef<'_>> for Link {
    fn from(element: ElementRef<'_>) -> Self {
        let href = element.attr("href").unwrap_or("").to_string();
        let text = element.text().collect();

        Self { href, text }
    }
}
