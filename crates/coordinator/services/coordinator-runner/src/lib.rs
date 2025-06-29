use axum::Router;
use dashmap::DashMap;
use lib_coordinator_core::PendingResponses;
use lib_coordinator_grpc::{grpc_server_service::CoordinatorServiceImpl, run_grpc_server};
use lib_coordinator_rest::build_rest_router;
use lib_coordinator_ws::build_ws_router;
use proto::generated::Envelope;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::broadcast;
use tracing::info;

pub async fn run(
    grpc_coordinator_addr: &str,
    api_addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_with_ready_callback(grpc_coordinator_addr, api_addr, || {}).await
}

pub async fn run_with_ready_callback<F>(
    grpc_coordinator_addr: &str,
    api_addr: &str,
    ready_callback: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: FnOnce() + Send + 'static,
{
    let grpc_coordinator_addr = grpc_coordinator_addr.parse()?;
    let api_addr: SocketAddr = api_addr.parse()?;

    let (server_cmd_tx, _) = broadcast::channel(2048);

    let clients: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>> =
        Arc::new(DashMap::new());

    let pending: PendingResponses = Arc::new(DashMap::new());

    let coordinator_service =
        CoordinatorServiceImpl::new(clients.clone(), server_cmd_tx.clone(), pending.clone());

    info!(
        "gRPC Conversation server listening on {}",
        grpc_coordinator_addr
    );
    info!("HTTP (WS+REST) server listening on {}", api_addr);

    let ws_router = build_ws_router(server_cmd_tx.clone(), clients.clone(), pending.clone());
    let rest_router = build_rest_router(server_cmd_tx.clone(), pending.clone());
    let app = Router::new().merge(ws_router).merge(rest_router);

    let http_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(api_addr).await?;
        ready_callback();

        axum::serve(listener, app.into_make_service()).await?;
        Ok(()) as Result<(), Box<dyn std::error::Error + Send + Sync>>
    });

    let grpc_handle =
        tokio::spawn(
            async move { run_grpc_server(coordinator_service, grpc_coordinator_addr).await },
        );

    let _ = tokio::try_join!(grpc_handle, http_handle)?;

    Ok(())
}
