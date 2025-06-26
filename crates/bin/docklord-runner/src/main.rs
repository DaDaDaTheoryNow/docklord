use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long = "type", value_parser = ["node", "coordinator"], help = "Launch type: node or coordinator")]
    mode: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.mode.as_str() {
        "coordinator" => {
            info!("Running Coordinator");
            println!("");

            coordinator_runner::run().await?;
        }
        "node" => {
            info!("Running Node");
            println!("");

            node_runner::run().await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
