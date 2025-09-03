use anyhow::{anyhow, Result};
use libxml::{parser, tree::document, xpath};
use scraper::{selector::ToCss, ElementRef, Html, Selector};
use serde::{Deserialize, Deserializer};
use tokio_util::sync::CancellationToken;

use crate::request;

#[derive(Debug, Deserialize, Clone)]
pub struct Page {
    pub name: String,
    pub url: String,
    pub extract: Extract,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Extract {
    #[serde(deserialize_with = "deserialize_selector")]
    CSSPlain(Selector),
    CSSDetailed {
        #[serde(deserialize_with = "deserialize_selector")]
        container: Selector,

        #[serde(
            rename = "href-tag",
            deserialize_with = "deserialize_selector"
        )]
        href: Selector,

        #[serde(
            rename = "text-tag",
            deserialize_with = "deserialize_selector"
        )]
        text: Selector,
    },
    XPathPlain(XPath),
    XPathDetailed {
        container: XPath,

        #[serde(rename = "href-path")]
        href: XPath,

        #[serde(rename = "text-path")]
        text: XPath,
    },
}

impl std::fmt::Display for Extract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Extract::CSSPlain(selector) => {
                write!(f, "CSS {{ {:?} }}", selector.to_css_string())
            }
            Extract::CSSDetailed {
                container,
                href: href_tag,
                text: text_tag,
            } => {
                write!(
                    f,
                    "CSS {{ \
                     container = {:?}, \
                     href_tag = {:?}, \
                     text_tag = {:?} \
                     }}",
                    container.to_css_string(),
                    href_tag.to_css_string(),
                    text_tag.to_css_string()
                )
            }
            Extract::XPathPlain(xpath) => {
                write!(f, "XPath {{ {xpath:?} }}")
            }
            Extract::XPathDetailed {
                container,
                href,
                text,
            } => {
                write!(
                    f,
                    "XPath {{ \
                     container = {:?}, \
                     href = {:?}, \
                     text = {:?} \
                     }}",
                    container, href, text
                )
            }
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "UncheckedXPath")]
pub struct XPath(String);

#[derive(Debug, Deserialize, Clone)]
#[serde(rename = "XPath")]
pub struct UncheckedXPath(String);

impl TryFrom<UncheckedXPath> for XPath {
    type Error = anyhow::Error;

    fn try_from(
        unchecked: UncheckedXPath,
    ) -> std::result::Result<Self, Self::Error> {
        let expr = unchecked.0;
        let mut empty_ctx = xpath::Context::new(
            &document::Document::new()
                .expect("empty document should be constructible"),
        )
        .expect("empty document should have valid context");

        if empty_ctx.findnodes(&expr, None).is_err() {
            Err(anyhow!("invalid XPath expression: {expr:?}"))
        } else {
            Ok(XPath(expr))
        }
    }
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
    pub async fn request(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Link>> {
        let body = request::get(&self.url, cancellation_token)
            .await?
            .text()
            .await?;

        match &self.extract {
            Extract::CSSPlain(selector) => {
                log::debug!(
                    target: &self.name,
                    "CSSPlain {{ {:?} }}",
                    selector.to_css_string()
                );

                Ok(Html::parse_fragment(&body)
                    .select(selector)
                    .map(Link::from)
                    .collect())
            }
            Extract::CSSDetailed {
                container,
                href: href_tag,
                text: text_tag,
            } => {
                log::debug!(
                    target: &self.name,
                    "CSSDetailed {{ \
                     container: {:?}, \
                     href: {:?}, \
                     text: {:?} \
                     }}",
                    container.to_css_string(),
                    href_tag.to_css_string(),
                    text_tag.to_css_string()
                );

                Ok(Html::parse_fragment(&body)
                    .select(container)
                    .map(|root| {
                        let href = root
                            .select(href_tag)
                            .next()
                            .map(|element| {
                                element.attr("href").unwrap_or("")
                            })
                            .unwrap_or("")
                            .to_string();

                        let text = root
                            .select(text_tag)
                            .next()
                            .map(|element| {
                                element.text().collect::<String>()
                            })
                            .unwrap_or_default()
                            .to_string();

                        Link { href, text }
                    })
                    .collect())
            }
            Extract::XPathPlain(XPath(expr)) => {
                log::debug!(
                    target: &self.name,
                    "XPathPlain {{ {expr:?} }}"
                );

                let html = parser::Parser::default_html()
                    .parse_string(&body)?;
                let nodes = xpath::Context::new(&html)
                    .map_err(|()| anyhow!("XPath context"))?
                    .findnodes(expr, None);

                log::debug!(target: &self.name, "nodes: {nodes:?}");

                Ok(nodes
                    .map_err(|()| anyhow!("XPath findnodes: {expr:?}"))?
                    .iter()
                    .map(Link::from)
                    .collect())
            }
            Extract::XPathDetailed {
                container: XPath(container),
                href: XPath(href_path),
                text: XPath(text_path),
            } => {
                log::debug!(
                    target: &self.name,
                    "XPathDetailed {{ \
                     container: {container:?}, \
                     href: {href_path:?}, \
                     text: {text_path:?} \
                     }}"
                );

                let html = parser::Parser::default_html()
                    .parse_string(&body)?;
                let mut ctx = xpath::Context::new(&html)
                    .map_err(|()| anyhow!("XPath context"))?;
                let nodes = ctx.findnodes(container, None);

                log::debug!(target: &self.name, "nodes: {nodes:?}");

                Ok(nodes
                    .map_err(|()| {
                        anyhow!("XPath findnodes: {container:?}")
                    })?
                    .iter()
                    .map(|root| {
                        let href = ctx
                            .findvalue(href_path, Some(root))
                            .unwrap_or_else(|()| String::new())
                            .to_string();

                        let text = ctx
                            .findvalue(text_path, Some(root))
                            .unwrap_or_else(|()| String::new())
                            .to_string();

                        Link { href, text }
                    })
                    .collect())
            }
        }
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

impl From<&libxml::tree::node::Node> for Link {
    fn from(element: &libxml::tree::node::Node) -> Self {
        let href = element.get_attribute("href").unwrap_or_default();
        let text = element.get_content();

        Self { href, text }
    }
}
