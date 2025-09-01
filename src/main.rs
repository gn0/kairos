use anyhow::Result;
use clap::ArgAction;
use clap::Parser;
use std::time::Duration;

mod collection;
mod config;
mod database;
mod page;
mod pushover;
mod request;

use crate::collection::Collection;
use crate::config::Config;
use crate::database::Database;
use crate::pushover::Pushover;

/// Command-line interface to open-webui.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Path to configuration file.
    #[arg(long, short)]
    config: String,

    /// Set log level (-v for info, -vv for debug, -vvv for trace).
    #[arg(long, short, action = ArgAction::Count)]
    verbose: u8,
}

async fn send_notification(
    collection: &Collection,
    pushover: &Pushover,
) -> Result<()> {
    let mut n_pages = 0;
    let mut chunks = Vec::new();

    // There are two cases that determine how the message is composed:
    //
    // 1. There are at most three pages with new links.
    //    - Mention the new link count for each page.
    //
    // 2. There are four or more pages with new links.
    //    - Mention the new link counts for the first two pages.
    //    - Only mention the total count for the remaining pages.
    //

    for (page_name, n_new_links) in
        collection.counter.iter().filter(|(_, x)| **x > 0)
    {
        n_pages += 1;

        if n_pages <= 3 {
            chunks.push(format!("{n_new_links} for {page_name}"));
        }
    }

    let message = match n_pages {
        1 => format!("{}.", chunks[0]),
        2 => format!("{} and {}.", chunks[0], chunks[1]),
        3 => {
            format!("{}, {}, and {}.", chunks[0], chunks[1], chunks[2])
        }
        _ => {
            if let Some(chunk) = chunks.get_mut(2) {
                *chunk = format!(
                    "and some more for {} other pages.",
                    n_pages - 2
                );
            }

            chunks.join(", ")
        }
    };

    let title = {
        let x = collection.stats.n_new_links;

        if x > 1 {
            format!("{x} new links")
        } else {
            format!("{x} new link")
        }
    };

    pushover.send(&message, Some(&title)).await?;

    Ok(())
}

async fn process(args: &Args) -> Result<()> {
    let config = Config::load(&args.config)?;
    let database = Database::try_new(&config.database)?;

    const DUR_24_HOURS: u64 = 24 * 60 * 60;
    let mut interval =
        tokio::time::interval(Duration::from_secs(DUR_24_HOURS));

    loop {
        interval.tick().await;

        let collection =
            Collection::try_new(&config.page, &database).await?;

        if collection.stats.n_new_links > 0
            && let Some(ref pushover) = config.pushover
        {
            send_notification(&collection, pushover).await?;
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let max_level = match args.verbose {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        3 => log::LevelFilter::Trace,
        _ => {
            eprintln!("error: too many occurrences of --verbose/-v");
            std::process::exit(1);
        }
    };

    env_logger::Builder::new().filter_level(max_level).init();

    match process(&args).await {
        Ok(_) => std::process::exit(0),
        Err(x) => {
            log::error!("{x}");
            std::process::exit(1);
        }
    }
}
