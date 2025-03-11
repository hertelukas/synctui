use clap::Parser;
use color_eyre::eyre;
use synctui::Client;

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
    Ok(())
}
