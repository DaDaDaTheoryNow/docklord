use axum::{Extension, Json, extract::Query, response::IntoResponse};
use lib_coordinator_core::{PendingResponses, ServerRequestByUser};
use proto::generated::{
    Envelope, GetNodeContainersWithStatus, NodeCommand, RequestType, envelope::Payload,
    node_command,
};
use serde_json::json;
use tokio::sync::{broadcast, oneshot};
use tracing::error;
use uuid::Uuid;

use crate::AuthParams;

const GET_CONTAINERS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub async fn get_containers(
    Extension(server_tx): Extension<broadcast::Sender<ServerRequestByUser>>,
    Extension(pending): Extension<PendingResponses>,
    Query(query): Query<AuthParams>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let (response_tx, response_rx) = oneshot::channel();

    // Register a pending response for this request
    pending.insert(
        (
            request_id.clone(),
            RequestType::GetContainersWithStatus as i32,
        ),
        response_tx,
    );

    // Build the command envelope to ask the node for containers with status
    let envelope = Envelope {
        payload: Some(Payload::NodeCommand(NodeCommand {
            kind: Some(node_command::Kind::GetNodeContainersWithStatus(
                GetNodeContainersWithStatus {
                    request_id: request_id.clone(),
                },
            )),
        })),
    };

    // Send the request to the node via broadcast
    let send_request = server_tx
        .send(ServerRequestByUser {
            id: query.node_id.clone(),
            password: query.password.clone(),
            envelope,
        })
        .map(|_| ());

    if let Err(e) = send_request {
        error!("Failed to send server request: {}", e);
        pending.remove(&(
            request_id.clone(),
            RequestType::GetContainersWithStatus as i32,
        ));
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to send request to server",
        )
            .into_response();
    }

    // Wait for the response from the node with a timeout
    match tokio::time::timeout(GET_CONTAINERS_TIMEOUT, response_rx).await {
        Ok(Ok(response)) => {
            // Parse containers with status from response
            let containers_with_status = extract_containers_with_status_from_response(&response);
            let body = json!({
                "id": request_id,
                "containers": containers_with_status,
            });
            (axum::http::StatusCode::OK, Json(body)).into_response()
        }
        Ok(Err(_)) => {
            let body = json!({
                "error": {
                    "message": "Response channel closed",
                    "data": {
                        "req_id": request_id,
                        "detail": "Node dropped oneshot channel"
                    }
                }
            });
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
        }
        Err(_) => {
            let body = json!({
                "error": {
                    "message": "No response from node",
                    "data": {
                        "req_id": request_id,
                        "detail": "Timeout waiting for node response"
                    }
                }
            });
            (axum::http::StatusCode::REQUEST_TIMEOUT, Json(body)).into_response()
        }
    }
}

fn extract_containers_with_status_from_response(response: &Envelope) -> Vec<serde_json::Value> {
    if let Some(proto::generated::envelope::Payload::NodeResponse(node_resp)) = &response.payload {
        if let Some(proto::generated::node_response::Kind::NodeContainersWithStatus(
            containers_msg,
        )) = &node_resp.kind
        {
            return containers_msg
                .containers
                .iter()
                .map(|container| {
                    json!({
                        "container_id": container.container_id,
                        "status": container.status,
                        "created": container.created,
                        "started_at": container.started_at,
                        "finished_at": container.finished_at,
                        "exit_code": container.exit_code,
                    })
                })
                .collect();
        }
    }
    vec![]
}
