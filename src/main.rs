use anyhow::Result;
use clap::ArgAction;
use clap::Parser;
use std::time::Duration;
use tokio::signal::unix::SignalKind;
use tokio_util::sync::CancellationToken;

mod collection;
mod config;
mod database;
mod page;
mod pushover;
mod request;

use crate::collection::Collection;
use crate::config::Config;
use crate::database::Database;
use crate::page::Page;
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
    cancellation_token: CancellationToken,
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

    pushover
        .send(&message, Some(&title), cancellation_token)
        .await?;

    Ok(())
}

async fn collect_and_notify(
    pages: &[Page],
    database: &Database,
    pushover: Option<&Pushover>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let collection = Collection::try_new(
        pages,
        database,
        cancellation_token.clone(),
    )
    .await?;

    if collection.stats.n_new_links > 0
        && let Some(x) = pushover
    {
        send_notification(&collection, x, cancellation_token).await?;
    }

    Ok(())
}

async fn process(args: &Args) -> Result<()> {
    let mut config = Config::load(&args.config)?;
    let database = Database::try_new(&config.database)?;

    let mut sighup = tokio::signal::unix::signal(SignalKind::hangup())?;
    let mut sigusr1 =
        tokio::signal::unix::signal(SignalKind::user_defined1())?;
    let mut current_task: Option<CancellationToken> = None;

    const DUR_24_HOURS: u64 = 24 * 60 * 60;
    let mut interval =
        tokio::time::interval(Duration::from_secs(DUR_24_HOURS));

    loop {
        tokio::select! {
            _ = sighup.recv() => {
                log::info!("reloading config from {:?}", args.config);
                match Config::load(&args.config) {
                    Ok(x) => config = x,
                    Err(x) => log::error!("{x}"),
                }
            },
            _ = sigusr1.recv() => match current_task {
                Some(token) => {
                    log::info!("cancelling collection");
                    token.cancel();
                    current_task = None;
                }
                None => {
                    log::info!("no collection to cancel")
                }
            },
            _ = interval.tick() => {
                let pages = config.page.clone();
                let pushover = config.pushover.clone();
                let database = database.clone();

                if let Some(token) = current_task {
                    log::info!(
                        "collection still in process; cancelling"
                    );
                    token.cancel();
                }

                let token = CancellationToken::new();
                let token_clone = token.clone();

                tokio::task::spawn(async move {
                    if let Err(x) = collect_and_notify(
                        &pages,
                        &database,
                        pushover.as_ref(),
                        token_clone,
                    ).await {
                        log::error!("collection: {x}");
                    }
                });

                current_task = Some(token);
            },
        };
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
