use clap::ArgAction;
use clap::Parser;

mod config;
mod page;
mod pushover;

use crate::config::Config;

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

async fn process(args: &Args) -> Result<(), String> {
    let config = Config::load(&args.config)?;

    dbg!(&config);

    // TODO Implement Page::request.
    for page in config.page.iter() {
        for link in page.request().await?.iter() {
            println!("{}: [{}]({})", page.name, link.text, link.href);
        }
    }

    // TODO Implement a database backend.

    // TODO Implement Pushover::send.

    // TODO Implement a timer that calls Page::request for each page
    // once a day.  Implement exponential backoff?

    todo!()
}

#[tokio::main(flavor = "current_thread")]
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
