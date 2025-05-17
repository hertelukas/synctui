use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::{self, Context};
use serde::Serialize;
use syncthing_rs::Client;
use synctui::{AppConfig, start};
use tokio::{sync::broadcast, task};

#[derive(clap::ValueEnum, Clone, Debug, Serialize, Default)]
enum LevelFilter {
    Off,
    Error,
    Warn,
    Info,
    #[default]
    Debug,
    Trace,
}

impl From<LevelFilter> for log::LevelFilter {
    fn from(val: LevelFilter) -> Self {
        match val {
            LevelFilter::Off => log::LevelFilter::Off,
            LevelFilter::Error => log::LevelFilter::Error,
            LevelFilter::Warn => log::LevelFilter::Warn,
            LevelFilter::Info => log::LevelFilter::Info,
            LevelFilter::Debug => log::LevelFilter::Debug,
            LevelFilter::Trace => log::LevelFilter::Trace,
        }
    }
}

/// CLI wrapper around the syncthing API
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Syncthing API key
    #[arg(short, long)]
    api_key: Option<String>,

    /// Run only as CLI, do not start TUI
    #[arg(long)]
    cli: bool,

    /// Provide custom config path
    #[arg(short, long)]
    config: Option<String>,

    /// Set log level
    #[arg(short, long)]
    log_level: Option<LevelFilter>,

    /// Set path of log file
    #[arg(long, requires = "log_level")]
    log_file: Option<PathBuf>,
}

fn default_log_file_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|mut path| {
        path.push("synctui");
        path.push("log.txt");
        path
    })
}

fn setup_logging(path: PathBuf, level: log::LevelFilter) -> eyre::Result<()> {
    if let Some(parent_dir) = path.parent() {
        if !parent_dir.as_os_str().is_empty() {
            std::fs::create_dir_all(parent_dir).wrap_err_with(|| {
                format!(
                    "Failed to create parent directory '{}' while preparing log file",
                    parent_dir.display()
                )
            })?;
        }
    }

    let target_file = std::fs::File::create(&path)
        .wrap_err_with(|| format!("Failed to create log file at '{}'", path.display()))?;
    let target = Box::new(target_file);

    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(target))
        .filter(None, level)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let level = args.log_level;
    if let Some(level) = level {
        let path = args.log_file
            .or_else(default_log_file_path)
            .ok_or_else(|| eyre::eyre!("Failed to determine a log file path: No path specified via --log-file and could not determine a default path."))?;

        setup_logging(path, level.into())?;
    }
    let api_key = {
        match args.api_key {
            Some(key) => key,
            None => AppConfig::load(args.config)?.api_key,
        }
    };

    let client = Client::builder(&api_key).build()?;

    if args.cli {
        client.ping().await?;
        client.get_configuration().await?;

        let (tx_event, mut rx_event) = broadcast::channel(1);

        task::spawn(async move {
            if let Err(error) = client.get_events(tx_event, false).await {
                println!("Error: {error:?}");
            }
        });

        task::spawn(async move {
            while let Ok(event) = rx_event.recv().await {
                println!("{:#?}", event);
            }
        })
        .await?;
    } else {
        start(client).await?;
    }

    Ok(())
}
