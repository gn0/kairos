use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use std::ops::Add;

use crate::database::Database;
use crate::page::Page;

#[derive(Debug)]
pub struct Collection {
    pub stats: CollectionStats,
    pub counter: IndexMap<String, u64>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CollectionStats {
    pub n_pages: u64,
    pub n_links: u64,
    pub n_new_links: u64,
}

impl Collection {
    pub async fn try_new(
        pages: &[Page],
        database: &Database,
    ) -> Result<Self> {
        let collection_id = database.start_collection().await?;
        let mut counter = IndexMap::new();
        let mut page_tasks = Vec::new();

        log::info!("starting collection {collection_id}");

        for page in pages {
            counter.insert(page.name.clone(), 0);

            page_tasks.push((
                &page.name,
                tokio::task::spawn(collect_page(
                    page.clone(),
                    collection_id,
                    database.clone(),
                )),
            ));
        }

        let mut total: CollectionStats = Default::default();

        for (page_name, task) in page_tasks {
            let stats = task.await.context("collection")??;

            total = total + stats;

            match counter.entry(page_name.clone()) {
                entry @ indexmap::map::Entry::Occupied(_) => {
                    entry.and_modify(|x| *x += stats.n_new_links);
                }
                indexmap::map::Entry::Vacant(_) => {
                    bail!("collection: IndexMap error");
                }
            }
        }

        log::info!(
            "ending collection {} with {} new links",
            collection_id,
            total.n_new_links
        );

        database
            .end_collection(
                collection_id,
                total.n_pages,
                total.n_links,
                total.n_new_links,
            )
            .await?;

        // TODO Update `is_active` for each record in `links`.

        Ok(Self {
            stats: total,
            counter,
        })
    }
}

impl Add for CollectionStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            n_pages: self.n_pages + other.n_pages,
            n_links: self.n_links + other.n_links,
            n_new_links: self.n_new_links + other.n_new_links,
        }
    }
}

async fn collect_page(
    page: Page,
    collection_id: i64,
    database: Database,
) -> Result<CollectionStats> {
    let page_id = database.add_page(&page.url, &page.selector).await?;
    let mut n_links = 0;
    let mut n_new_links = 0;

    log::info!(target: &page.name, "page ID {page_id}");

    for link in page.request().await?.iter() {
        let mut is_new = false;
        n_links += 1;

        if !database
            .link_exists(page_id, &link.href, &link.text)
            .await?
        {
            is_new = true;
            n_new_links += 1;
        }

        let link_id =
            database.add_link(page_id, &link.href, &link.text).await?;

        if is_new {
            log::info!(
                target: &page.name,
                "new link {:?} {:?}",
                link.href,
                link.text
            );
        } else {
            log::info!(
                target: &page.name,
                "existing link {:?} {:?}",
                link.href,
                link.text
            );
        }

        database.add_link_collection(link_id, collection_id).await?;
    }

    Ok(CollectionStats {
        n_pages: 1,
        n_links,
        n_new_links,
    })
}
