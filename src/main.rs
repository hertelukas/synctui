use clap::Parser;
use color_eyre::eyre;
use synctui::{AppConfig, Client, start};
use tokio::{sync::mpsc, task};

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
}

fn setup_logging() {
    let target = Box::new(std::fs::File::create("log.txt").expect("Can't create file"));

    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(target))
        .filter(None, log::LevelFilter::Debug)
        .init();
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    setup_logging();
    let args = Args::parse();
    let api_key = {
        match args.api_key {
            Some(key) => key,
            None => AppConfig::load(args.config)?.api_key,
        }
    };
    let client = Client::new(&api_key)?;

    if args.cli {
        client.ping().await?;
        client.get_configuration().await?;

        let (tx_event, mut rx_event) = mpsc::channel(1);

        task::spawn(async move {
            if let Err(error) = client.get_events(tx_event).await {
                println!("Error: {error:?}");
            }
        });

        task::spawn(async move {
            while let Some(event) = rx_event.recv().await {
                println!("{:#?}", event);
            }
        })
        .await?;
    } else {
        start(client).await?;
    }

    Ok(())
}
