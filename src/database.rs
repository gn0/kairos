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
        let connection = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            #[rustfmt::skip]
            connection
                .blocking_lock()
                .execute(
                    "INSERT INTO collections (start_time) \
                     VALUES (DATETIME('now', 'utc'))",
                    (),
                )
                .context("database.add_collection: INSERT")?;

            Ok(connection.blocking_lock().last_insert_rowid())
        })
        .await?
    }

    pub async fn end_collection(
        &self,
        collection_id: i64,
        n_pages: u64,
        n_links: u64,
        n_new_links: u64,
    ) -> Result<()> {
        let connection = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            #[rustfmt::skip]
            connection
                .blocking_lock()
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
        .await?
    }

    pub async fn add_page(
        &self,
        url: &str,
        selector: &Selector,
    ) -> Result<i64> {
        let connection = self.connection.clone();
        let url = url.to_string();
        let selector_str = selector.to_css_string();

        tokio::task::spawn_blocking(move || {
            #[rustfmt::skip]
            connection
                .blocking_lock()
                .execute(
                    "INSERT OR IGNORE INTO pages (url, selector) \
                     VALUES (?1, ?2)",
                    (&url, &selector_str),
                )
                .context("database.add_page: INSERT OR IGNORE")?;

            #[rustfmt::skip]
            let page_id = connection
                .blocking_lock()
                .query_row(
                    "SELECT id FROM pages \
                     WHERE url = ?1 AND selector = ?2",
                    (&url, &selector_str),
                    |row| row.get(0),
                )
                .context("database.add_page: SELECT")?;

            Ok(page_id)
        })
        .await?
    }

    pub async fn add_link(
        &self,
        page_id: i64,
        href: &str,
        text: &str,
    ) -> Result<i64> {
        let connection = self.connection.clone();
        let href = href.to_string();
        let text = text.to_string();

        tokio::task::spawn_blocking(move || {
            #[rustfmt::skip]
            connection
            .blocking_lock()
            .execute(
                "INSERT OR IGNORE INTO links (page_id, href, text) \
                 VALUES (?1, ?2, ?3)",
                (page_id, &href, &text),
            )
            .context("database.add_link: INSERT OR IGNORE")?;

            #[rustfmt::skip]
            let link_id = connection
                .blocking_lock()
                .query_row(
                    "SELECT id FROM links \
                     WHERE page_id = ?1 AND href = ?2 AND text = ?3",
                    (page_id, &href, &text),
                    |row| row.get(0),
                )
                .context("database.add_link: SELECT")?;

            Ok(link_id)
        })
        .await?
    }

    pub async fn link_exists(
        &self,
        page_id: i64,
        href: &str,
        text: &str,
    ) -> Result<bool> {
        let connection = self.connection.clone();
        let href = href.to_string();
        let text = text.to_string();

        tokio::task::spawn_blocking(move || {
            #[rustfmt::skip]
            let count: i64 = connection
                .blocking_lock()
                .query_row(
                    "SELECT COUNT(*) FROM links \
                     WHERE page_id = ?1 AND href = ?2 AND text = ?3",
                    (page_id, &href, &text),
                    |row| row.get(0),
                )
                .context("database.link_exists: SELECT")?;

            Ok(count > 0)
        })
        .await?
    }

    pub async fn add_link_collection(
        &self,
        link_id: i64,
        collection_id: i64,
    ) -> Result<()> {
        let connection = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            #[rustfmt::skip]
            connection
            .blocking_lock()
            .execute(
                "INSERT INTO links_collections \
                 (link_id, collection_id, timestamp) \
                 VALUES (?1, ?2, DATETIME('now', 'utc'))",
                (link_id, collection_id),
            )
            .context("database.add_link_collection: INSERT")?;

            Ok(())
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn add_page_does_not_add_duplicate() {
        let db = Database::try_new(":memory:").unwrap();
        let sel = Selector::parse("a").unwrap();

        let id_a = db.add_page("http://foo.bar", &sel).await.unwrap();
        let id_b = db.add_page("http://foo.bar", &sel).await.unwrap();

        assert_eq!(id_a, id_b);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn add_page_accounts_for_url() {
        let db = Database::try_new(":memory:").unwrap();
        let sel = Selector::parse("a").unwrap();

        let id_a = db.add_page("http://foo/bar", &sel).await.unwrap();
        let id_b = db.add_page("http://foo/baz", &sel).await.unwrap();

        assert_ne!(id_a, id_b);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn add_page_accounts_for_selector() {
        let db = Database::try_new(":memory:").unwrap();
        let sel_a = Selector::parse("a[href^='/foo']").unwrap();
        let sel_b = Selector::parse("a[href^='/bar']").unwrap();

        let id_a = db.add_page("http://foo.bar", &sel_a).await.unwrap();
        let id_b = db.add_page("http://foo.bar", &sel_b).await.unwrap();

        assert_ne!(id_a, id_b);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn add_link_requires_valid_page_id() {
        let db = Database::try_new(":memory:").unwrap();
        let nonexistent = 1;

        assert!(db.add_link(nonexistent, "/foo", "bar").await.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn add_link_does_not_add_duplicate() {
        let db = Database::try_new(":memory:").unwrap();
        let sel = Selector::parse("a").unwrap();
        let page_id =
            db.add_page("http://foo.bar", &sel).await.unwrap();

        let id_a = db.add_link(page_id, "/foo", "bar").await.unwrap();
        let id_b = db.add_link(page_id, "/foo", "bar").await.unwrap();

        assert_eq!(id_a, id_b);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn link_exists_works() {
        let db = Database::try_new(":memory:").unwrap();
        let sel = Selector::parse("a").unwrap();
        let page_id =
            db.add_page("http://foo.bar", &sel).await.unwrap();

        assert!(!db.link_exists(page_id, "/foo", "bar").await.unwrap());
        assert!(!db.link_exists(page_id, "/bar", "baz").await.unwrap());

        db.add_link(page_id, "/foo", "bar").await.unwrap();

        assert!(db.link_exists(page_id, "/foo", "bar").await.unwrap());
        assert!(!db.link_exists(page_id, "/bar", "baz").await.unwrap());

        db.add_link(page_id, "/lorem", "ipsum").await.unwrap();

        assert!(db.link_exists(page_id, "/foo", "bar").await.unwrap());
        assert!(!db.link_exists(page_id, "/bar", "baz").await.unwrap());

        db.add_link(page_id, "/bar", "baz").await.unwrap();

        assert!(db.link_exists(page_id, "/foo", "bar").await.unwrap());
        assert!(db.link_exists(page_id, "/bar", "baz").await.unwrap());
    }
}
