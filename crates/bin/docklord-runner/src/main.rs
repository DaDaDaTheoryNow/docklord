use clap::Parser;
use std::env;
use tracing::{error, info};

mod gen_credentials;
use gen_credentials::{generate_node_id, generate_secure_password};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long = "type", value_parser = ["node", "coordinator", "self-hosted"], help = "Launch type: node, coordinator, or self-hosted (node with built-in coordinator)")]
    mode: String,

    // Coordinator options
    #[arg(long, help = "gRPC port for (node-coordinator communication)")]
    grpc_port: Option<u16>,

    #[arg(long, help = "API port for coordinator (user API)")]
    api_port: Option<u16>,

    // Node options
    #[arg(long, help = "Coordinator gRPC address")]
    coordinator_addr: Option<String>,

    #[arg(long, help = "Node ID (auto-generated if not specified)")]
    node_id: Option<String>,

    #[arg(long, help = "Node password (auto-generated if not specified)")]
    password: Option<String>,
}

fn get_port_from_env_or_default(env_var: &str, default: u16) -> u16 {
    env::var(env_var)
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(default)
}

fn get_coordinator_addr_from_env_or_default() -> String {
    env::var("COORDINATOR_ADDR").unwrap_or_else(|_| "http://localhost:50051".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .init();

    let cli = Cli::parse();

    // Get ports from environment variables or CLI args or defaults
    let grpc_port = cli
        .grpc_port
        .unwrap_or_else(|| get_port_from_env_or_default("GRPC_PORT", 50051));
    let api_port = cli
        .api_port
        .unwrap_or_else(|| get_port_from_env_or_default("API_PORT", 3000));

    // Get coordinator address from environment or CLI args
    let coordinator_addr = cli
        .coordinator_addr
        .unwrap_or_else(get_coordinator_addr_from_env_or_default);

    // Generate node_id and password if they do not exist
    let node_id = cli.node_id.unwrap_or_else(generate_node_id);
    let password = cli.password.unwrap_or_else(generate_secure_password);

    match cli.mode.as_str() {
        "coordinator" => {
            info!("Running Coordinator");
            info!("gRPC port: {}", grpc_port);
            info!("API port: {}", api_port);
            println!("");

            let grpc_addr = format!("0.0.0.0:{}", grpc_port);
            let api_addr = format!("0.0.0.0:{}", api_port);

            coordinator_runner::run(&grpc_addr, &api_addr).await?;
        }
        "node" => {
            info!("Running Node");
            info!("Coordinator address: {}", coordinator_addr);
            println!("");

            node_runner::run(&coordinator_addr, &node_id, &password).await?;
        }
        "self-hosted" => {
            info!("Running Self-Hosted Node (Coordinator + Node)");
            println!("");

            info!("gRPC port: {}", grpc_port);
            info!("API port: {}", api_port);
            println!("");

            let grpc_addr = format!("0.0.0.0:{}", grpc_port);
            let api_addr = format!("0.0.0.0:{}", api_port);
            let local_coordinator_addr = format!("http://localhost:{}", grpc_port);

            let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

            let coordinator_handle = tokio::spawn(async move {
                coordinator_runner::run_with_ready_callback(&grpc_addr, &api_addr, move || {
                    let _ = ready_tx.send(());
                })
                .await
            });

            let _ = ready_rx.await;
            info!("Coordinator is ready, starting node...");
            println!("");

            let node_handle = tokio::spawn(async move {
                node_runner::run(&local_coordinator_addr, &node_id, &password).await
                // node_runner::run(&local_coordinator_addr, "1", "123").await
            });

            tokio::select! {
                result = coordinator_handle => {
                    if let Err(e) = result {
                        error!("Coordinator failed: {:?}", e);
                    }
                }
                result = node_handle => {
                    if let Err(e) = result {
                        error!("Node failed: {:?}", e);
                    }
                }
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
