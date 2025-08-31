use clap::ArgAction;
use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

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
) -> Result<(), String> {
    let mut n_pages = 0;
    let mut chunks = Vec::new();

    for (page_name, n_new_links) in collection.counter.iter() {
        n_pages += 1;

        if n_pages <= 2 {
            chunks.push(format!("{n_new_links} for {page_name}"));
        }
    }

    if n_pages > 2 {
        chunks.push(format!(
            "and some more for {} other pages.",
            n_pages - 2
        ));
    }

    let message = match n_pages {
        1 => format!("{}.", chunks[0]),
        2 => format!("{} and {}.", chunks[0], chunks[1]),
        _ => chunks.join(", "),
    };

    pushover
        .send(
            &message,
            Some(&format!(
                "{} new links",
                collection.stats.n_new_links
            )),
        )
        .await?;

    Ok(())
}

async fn process(args: &Args) -> Result<(), String> {
    let config = Config::load(&args.config)?;
    let database =
        Arc::new(Mutex::new(Database::try_new(&config.database)?));

    const DUR_24_HOURS: u64 = 24 * 60 * 60;
    let mut interval =
        tokio::time::interval(Duration::from_secs(DUR_24_HOURS));

    loop {
        interval.tick().await;

        let collection =
            Collection::try_new(&config.page, database.clone()).await?;

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
