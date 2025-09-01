use anyhow::{Context, Result};
use rusqlite::Connection;
use scraper::{selector::ToCss, Selector};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl Database {
    const SCHEMA: &str = include_str!("schema.sql");

    pub fn try_new(path: impl AsRef<Path>) -> Result<Self> {
        let connection = tokio::task::block_in_place(move || {
            Connection::open(path)
        })?;

        connection
            .execute_batch(Self::SCHEMA)
            .context("database schema")?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub async fn start_collection(&self) -> Result<i64> {
        tokio::task::block_in_place(async move || {
            #[rustfmt::skip]
            self.connection
                .lock()
                .await
                .execute(
                    "INSERT INTO collections (start_time) \
                     VALUES (DATETIME('now', 'utc'))",
                    (),
                )
                .context("database.add_collection: INSERT")?;

            Ok(self.connection.lock().await.last_insert_rowid())
        })
        .await
    }

    pub async fn end_collection(
        &self,
        collection_id: i64,
        n_pages: u64,
        n_links: u64,
        n_new_links: u64,
    ) -> Result<()> {
        tokio::task::block_in_place(async move || {
            #[rustfmt::skip]
            self.connection
                .lock()
                .await
                .execute(
                    "UPDATE collections \
                     SET end_time = DATETIME('now', 'utc'), \
                     n_pages = ?1, \
                     n_links = ?2, \
                     n_new_links = ?3 \
                     WHERE id = ?4",
                    (n_pages, n_links, n_new_links, collection_id),
                )
                .context("database.end_collection: INSERT")?;

            Ok(())
        })
        .await
    }

    pub async fn add_page(
        &self,
        url: &str,
        selector: &Selector,
    ) -> Result<i64> {
        tokio::task::block_in_place(async move || {
            let selector_str = selector.to_css_string();

            #[rustfmt::skip]
            self.connection
                .lock()
                .await
                .execute(
                    "INSERT OR IGNORE INTO pages (url, selector) \
                     VALUES (?1, ?2)",
                    (url, &selector_str),
                )
                .context("database.add_page: INSERT OR IGNORE")?;

            #[rustfmt::skip]
            let page_id = self
                .connection
                .lock()
                .await
                .query_row(
                    "SELECT id FROM pages \
                     WHERE url = ?1 AND selector = ?2",
                    (url, selector.to_css_string()),
                    |row| row.get(0),
                )
                .context("database.add_page: SELECT")?;

            Ok(page_id)
        })
        .await
    }

    pub async fn add_link(
        &self,
        page_id: i64,
        href: &str,
        text: &str,
    ) -> Result<i64> {
        tokio::task::block_in_place(async move || {
            #[rustfmt::skip]
            self
            .connection
            .lock()
            .await
            .execute(
                "INSERT OR IGNORE INTO links (page_id, href, text) \
                 VALUES (?1, ?2, ?3)",
                (page_id, href, text),
            )
            .context("database.add_link: INSERT OR IGNORE")?;

            #[rustfmt::skip]
            let link_id = self
                .connection
                .lock()
                .await
                .query_row(
                    "SELECT id FROM links \
                     WHERE page_id = ?1 AND href = ?2 AND text = ?3",
                    (page_id, href, text),
                    |row| row.get(0),
                )
                .context("database.add_link: SELECT")?;

            Ok(link_id)
        })
        .await
    }

    pub async fn link_exists(
        &self,
        page_id: i64,
        href: &str,
        text: &str,
    ) -> Result<bool> {
        tokio::task::block_in_place(async move || {
            #[rustfmt::skip]
            let count: i64 = self
                .connection
                .lock()
                .await
                .query_row(
                    "SELECT COUNT(*) FROM links \
                     WHERE page_id = ?1 AND href = ?2 AND text = ?3",
                    (page_id, href, text),
                    |row| row.get(0),
                )
                .context("database.link_exists: SELECT")?;

            Ok(count > 0)
        })
        .await
    }

    pub async fn add_link_collection(
        &self,
        link_id: i64,
        collection_id: i64,
    ) -> Result<()> {
        tokio::task::block_in_place(async move || {
            #[rustfmt::skip]
            self
            .connection
            .lock()
            .await
            .execute(
                "INSERT INTO links_collections \
                 (link_id, collection_id, timestamp) \
                 VALUES (?1, ?2, DATETIME('now', 'utc'))",
                (link_id, collection_id),
            )
            .context("database.add_link_collection: INSERT")?;

            Ok(())
        })
        .await
    }
}
