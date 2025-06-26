use axum::{Extension, Router, routing::get};
use dashmap::DashMap;
use lib_coordinator_core::PendingResponses;
use proto::generated::Envelope;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::ws_observe_containers::{self};

pub fn build_ws_router(
    server_cmd_tx: broadcast::Sender<lib_coordinator_core::ServerRequestByUser>,
    clients: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>,
    pending: PendingResponses,
) -> Router {
    Router::new()
        .route(
            "/observe-containers",
            get(ws_observe_containers::handle_ws_connection),
        )
        .layer(Extension(server_cmd_tx.clone()))
        .layer(Extension(clients.clone()))
        .layer(Extension(pending.clone()))
}
