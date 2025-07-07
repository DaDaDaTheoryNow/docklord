use axum::{
    Extension, Json,
    extract::{Path, Query},
    response::IntoResponse,
};
use lib_coordinator_core::{PendingResponses, ServerRequestByUser};
use proto::generated::{
    Envelope, GetContainerStatus, NodeCommand, RequestType, envelope::Payload, node_command,
};
use serde_json::json;
use tokio::sync::{broadcast, oneshot};
use tracing::error;
use uuid::Uuid;

use crate::{ApiError, ApiErrorDetail, AuthParams};

const GET_CONTAINER_STATUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub async fn get_container_status(
    Path(container_id): Path<String>,
    Extension(server_tx): Extension<broadcast::Sender<ServerRequestByUser>>,
    Extension(pending): Extension<PendingResponses>,
    Query(query): Query<AuthParams>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let (response_tx, response_rx) = oneshot::channel();

    // Register a pending response for this request
    pending.insert(
        (request_id.clone(), RequestType::GetContainerStatus as i32),
        response_tx,
    );

    // Build the command envelope to ask the node for container status
    let envelope = Envelope {
        payload: Some(Payload::NodeCommand(NodeCommand {
            kind: Some(node_command::Kind::GetContainerStatus(GetContainerStatus {
                request_id: request_id.clone(),
                container_id: container_id.clone(),
            })),
        })),
    };

    // Send the request to the node via broadcast
    let send_result = server_tx
        .send(ServerRequestByUser {
            id: query.node_id.clone(),
            password: query.password.clone(),
            envelope,
        })
        .map(|_| ());

    if let Err(e) = send_result {
        error!("Failed to send server request: {}", e);
        pending.remove(&(request_id.clone(), RequestType::GetContainerStatus as i32));
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to send request to server",
        )
            .into_response();
    }

    // Wait for the response from the node with a timeout
    match tokio::time::timeout(GET_CONTAINER_STATUS_TIMEOUT, response_rx).await {
        Ok(Ok(response)) => {
            if let Some(err_msg) = extract_node_error_from_response(&response) {
                pending.remove(&(request_id.clone(), RequestType::GetContainerStatus as i32));
                let err = ApiError {
                    req_uuid: request_id.clone(),
                    error: ApiErrorDetail {
                        message: "Node error".to_string(),
                        detail: err_msg,
                    },
                };
                return (axum::http::StatusCode::BAD_REQUEST, Json(err)).into_response();
            }

            let container_status = extract_container_status_from_response(&response);
            let body = json!({
                "id": request_id,
                "container_id": container_id,
                "status": container_status,
            });
            (axum::http::StatusCode::OK, Json(body)).into_response()
        }
        Ok(Err(_)) => {
            pending.remove(&(request_id.clone(), RequestType::GetContainerStatus as i32));
            let err = ApiError {
                req_uuid: request_id.clone(),
                error: ApiErrorDetail {
                    message: "Response channel closed".to_string(),
                    detail: "Node dropped oneshot channel".to_string(),
                },
            };
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(err)).into_response()
        }
        Err(_) => {
            pending.remove(&(request_id.clone(), RequestType::GetContainerStatus as i32));
            let err = ApiError {
                req_uuid: request_id.clone(),
                error: ApiErrorDetail {
                    message: "Timeout waiting for node response".to_string(),
                    detail: "Timeout waiting for node response".to_string(),
                },
            };

            (axum::http::StatusCode::REQUEST_TIMEOUT, Json(err)).into_response()
        }
    }
}

fn extract_container_status_from_response(response: &Envelope) -> Option<serde_json::Value> {
    if let Some(proto::generated::envelope::Payload::NodeResponse(node_resp)) = &response.payload {
        if let Some(proto::generated::node_response::Kind::ContainerStatus(status)) =
            &node_resp.kind
        {
            return Some(json!({
                "status": status.status,
                "created": status.created,
                "started_at": status.started_at,
                "finished_at": status.finished_at,
                "exit_code": status.exit_code,
            }));
        }
    }
    None
}

fn extract_node_error_from_response(response: &Envelope) -> Option<String> {
    if let Some(proto::generated::envelope::Payload::NodeResponse(node_resp)) = &response.payload {
        if let Some(proto::generated::node_response::Kind::Error(err)) = &node_resp.kind {
            return Some(err.message.clone());
        }
    }
    None
}
