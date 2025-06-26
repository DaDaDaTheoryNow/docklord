use axum::{Extension, Router, routing::get};
use lib_coordinator_core::{PendingResponses, ServerRequestByUser};
use tokio::sync::broadcast;

use crate::get_containers::get_containers;

pub fn build_rest_router(
    server_cmd_tx: broadcast::Sender<ServerRequestByUser>,
    pending: PendingResponses,
) -> Router {
    Router::new()
        .route("/api/containers", get(get_containers))
        .layer(Extension(server_cmd_tx))
        .layer(Extension(pending))
}
