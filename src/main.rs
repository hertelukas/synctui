use clap::Parser;
use color_eyre::eyre;
use synctui::Client;
use tokio::{sync::mpsc, task};

/// CLI wrapper around the syncthing API
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Syncthing API key
    #[arg(short, long)]
    api_key: String,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    let client = Client::new(&args.api_key)?;
    client.ping().await?;
    client.get_config().await?;

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

    Ok(())
}
