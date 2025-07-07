use axum::{
    Extension, Router,
    routing::{delete, get, post},
};
use lib_coordinator_core::{PendingResponses, ServerRequestByUser};
use tokio::sync::broadcast;

use crate::container_actions::{delete_container, start_container, stop_container};
use crate::container_logs::get_container_logs;
use crate::container_status::get_container_status;
use crate::get_containers::get_containers;

pub fn build_rest_router(
    server_cmd_tx: broadcast::Sender<ServerRequestByUser>,
    pending: PendingResponses,
) -> Router {
    Router::new()
        .route("/api/containers", get(get_containers))
        .route(
            "/api/containers/{container_id}/status",
            get(get_container_status),
        )
        .route(
            "/api/containers/{container_id}/start",
            post(start_container),
        )
        .route("/api/containers/{container_id}/stop", post(stop_container))
        .route("/api/containers/{container_id}", delete(delete_container))
        .route(
            "/api/containers/{container_id}/logs",
            get(get_container_logs),
        )
        .layer(Extension(server_cmd_tx))
        .layer(Extension(pending))
}
