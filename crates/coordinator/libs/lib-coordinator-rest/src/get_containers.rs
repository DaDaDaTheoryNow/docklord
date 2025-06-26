use axum::{Extension, Json, extract::Query, response::IntoResponse};
use lib_coordinator_core::{PendingResponses, ServerRequestByUser};
use proto::generated::{
    ClientCommand, Envelope, GetClientContainers, RequestType, client_command, envelope::Payload,
};
use serde_json::json;
use tokio::sync::{broadcast, oneshot};
use tracing::error;
use uuid::Uuid;

use crate::AuthParams;

const GET_CONTAINERS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub async fn get_containers(
    Extension(server_tx): Extension<broadcast::Sender<ServerRequestByUser>>,
    Extension(pending): Extension<PendingResponses>,
    Query(query): Query<AuthParams>,
) -> impl IntoResponse {
    let request_id = Uuid::new_v4().to_string();
    let (response_tx, response_rx) = oneshot::channel();

    // Register a pending response for this request
    pending.insert(
        (request_id.clone(), RequestType::GetContainers as i32),
        response_tx,
    );

    // Build the command envelope to ask the client for containers
    let envelope = Envelope {
        payload: Some(Payload::ClientCommand(ClientCommand {
            kind: Some(client_command::Kind::GetClientContainers(
                GetClientContainers {
                    request_id: request_id.clone(),
                },
            )),
        })),
    };

    // Send the request to the client via broadcast
    let send_result = server_tx
        .send(ServerRequestByUser {
            id: query.client_id.clone(),
            password: query.password.clone(),
            envelope,
        })
        .map(|_| ());

    if let Err(e) = send_result {
        error!("Failed to send server request: {}", e);
        pending.remove(&(request_id.clone(), RequestType::GetContainers as i32));
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to send request to server",
        )
            .into_response();
    }

    // Wait for the response from the client with a timeout
    match tokio::time::timeout(GET_CONTAINERS_TIMEOUT, response_rx).await {
        Ok(Ok(response)) => {
            // Парсим список контейнеров из ответа (например, из proto)
            let containers = extract_containers_from_response(&response);
            let body = json!({
                "id": request_id,
                "containers": containers,
            });
            (axum::http::StatusCode::OK, Json(body)).into_response()
        }
        Ok(Err(_)) => {
            let body = json!({
                "error": {
                    "message": "Response channel closed",
                    "data": {
                        "req_uuid": request_id,
                        "detail": "Client dropped oneshot channel"
                    }
                }
            });
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
        }
        Err(_) => {
            let body = json!({
                "error": {
                    "message": "No response from client",
                    "data": {
                        "req_uuid": request_id,
                        "detail": "Timeout waiting for client response"
                    }
                }
            });
            (axum::http::StatusCode::REQUEST_TIMEOUT, Json(body)).into_response()
        }
    }
}

fn extract_containers_from_response(response: &Envelope) -> Vec<String> {
    if let Some(proto::generated::envelope::Payload::ClientResponse(client_resp)) =
        &response.payload
    {
        if let Some(proto::generated::client_response::Kind::ClientContainers(containers_msg)) =
            &client_resp.kind
        {
            return containers_msg.containers.clone();
        }
    }
    vec![]
}
