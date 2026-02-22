use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = switchboard::cli::Cli::parse();
    if let Err(e) = switchboard::cmd::dispatch(cli).await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
