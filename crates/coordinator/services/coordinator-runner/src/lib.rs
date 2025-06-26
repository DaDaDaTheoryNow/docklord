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

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let grpc_addr = "[::1]:50051".parse()?;
    let ws_addr: SocketAddr = "[::1]:3000".parse()?;

    let (server_cmd_tx, _) = broadcast::channel(2048);

    let clients: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>> =
        Arc::new(DashMap::new());

    let pending: PendingResponses = Arc::new(DashMap::new());

    let coordinator_service =
        CoordinatorServiceImpl::new(clients.clone(), server_cmd_tx.clone(), pending.clone());

    info!("gRPC Conversation server listening on {}", grpc_addr);
    info!("HTTP (WS+REST) server listening on {}", ws_addr);

    let ws_router = build_ws_router(server_cmd_tx.clone(), clients.clone(), pending.clone());
    let rest_router = build_rest_router(server_cmd_tx.clone(), pending.clone());
    let app = Router::new().merge(ws_router).merge(rest_router);

    tokio::try_join!(run_grpc_server(coordinator_service, grpc_addr), async {
        let listener = tokio::net::TcpListener::bind(ws_addr).await?;
        axum::serve(listener, app.into_make_service()).await?;

        Ok(()) as Result<(), Box<dyn std::error::Error + Send + Sync>>
    })?;

    Ok(())
}
