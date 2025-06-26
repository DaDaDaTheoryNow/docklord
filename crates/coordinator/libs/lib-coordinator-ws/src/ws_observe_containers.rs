use axum::{
    extract::{
        Extension, Query,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use lib_coordinator_core::ServerRequestByUser;
use lib_coordinator_rest::AuthParams;
use proto::generated::{
    ClientCommand, Envelope, GetClientContainers, RequestType, client_command,
    client_response::Kind, envelope::Payload,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn handle_ws_connection(
    Query(auth_params): Query<AuthParams>,
    ws: WebSocketUpgrade,
    Extension(server_tx): Extension<broadcast::Sender<ServerRequestByUser>>,
    Extension(clients): Extension<Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            auth_params.client_id,
            auth_params.password,
            server_tx,
            clients,
        )
    })
}

async fn handle_socket(
    socket: WebSocket,
    client_id: String,
    password: String,
    server_tx: broadcast::Sender<ServerRequestByUser>,
    clients: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    info!("🔌 New WebSocket connection for client: {}", client_id);

    // Check if the client is registered
    let client_key = (client_id.clone(), password.clone());
    let Some(client_tx) = clients.get(&client_key).map(|g| g.value().clone()) else {
        error!("Client {} not registered", client_id);
        let _ = ws_sender.send(Message::Close(None)).await;
        return;
    };

    // Immediately send a request to get the current containers list
    if let Err(e) = send_get_containers(&server_tx, &client_id, &password).await {
        error!("Failed to send containers request: {}", e);
    }

    // Subscribe to container updates for this client
    let mut broadcast_rx = client_tx.subscribe();
    info!("📡 Containers observing for client: {}", client_id);

    // Main loop: handle both client and server messages
    loop {
        tokio::select! {
            // Handle incoming messages from the WebSocket client
            msg = ws_receiver.next() => {
                if !handle_client_message(msg, &mut ws_sender).await {
                    break;
                }
            }

            // Handle messages from the server (container updates)
            msg = broadcast_rx.recv() => {
                if !handle_server_message(msg, &mut ws_sender, &client_id).await {
                    break;
                }
            }
        }
    }

    info!("🔚 WebSocket session ended for {}", client_id);
}

// Helper to send a GetClientContainers command to the client
async fn send_get_containers(
    server_tx: &broadcast::Sender<ServerRequestByUser>,
    client_id: &str,
    password: &str,
) -> Result<(), broadcast::error::SendError<ServerRequestByUser>> {
    server_tx
        .send(ServerRequestByUser {
            id: client_id.to_string(),
            password: password.to_string(),
            envelope: Envelope {
                payload: Some(Payload::ClientCommand(ClientCommand {
                    kind: Some(client_command::Kind::GetClientContainers(
                        GetClientContainers {
                            request_id: Uuid::new_v4().to_string(),
                        },
                    )),
                })),
            },
        })
        .map(|_| ())
}

// Handle messages from the WebSocket client (pings, closes, etc.)
async fn handle_client_message(
    msg: Option<Result<Message, axum::Error>>,
    ws_sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> bool {
    match msg {
        Some(Ok(Message::Ping(payload))) => {
            // Respond to ping with pong
            if ws_sender.send(Message::Pong(payload)).await.is_err() {
                error!("Failed to send Pong");
                return true;
            }
            true
        }
        Some(Ok(Message::Close(frame))) => {
            info!("Client disconnected: {:?}", frame);
            false
        }
        Some(Ok(_)) => {
            // Ignore other message types
            true
        }
        Some(Err(e)) => {
            error!("WebSocket error: {:?}", e);
            true
        }
        None => {
            info!("Client connection closed");
            false
        }
    }
}

// Handle messages from the server (container updates) and send to WebSocket client
async fn handle_server_message(
    msg: Result<Envelope, broadcast::error::RecvError>,
    ws_sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    client_id: &str,
) -> bool {
    match msg {
        Ok(envelope) => {
            info!("{:?}", envelope);
            if let Some(Payload::ClientResponse(resp)) = envelope.payload {
                match resp.kind {
                    Some(Kind::ClientContainers(ref containers_msg)) => {
                        if let Some(ref rk) = containers_msg.request_key {
                            if rk.request_type == RequestType::GetContainers as i32
                                || rk.request_type == RequestType::UpdateContainerInfo as i32
                            {
                                let body = json!({
                                    "containers": containers_msg.containers,
                                });

                                if ws_sender
                                    .send(Message::Text(body.to_string().into()))
                                    .await
                                    .is_err()
                                {
                                    error!("Failed to send to client {}", client_id);
                                    return false;
                                }
                                return true;
                            }
                        } else {
                            return true;
                        }
                    }
                    _ => {
                        let body = json!({
                            "error": {
                                "message": "Unknown response type",
                                "data": {
                                    "detail": format!("Unexpected response: {:?}", resp.kind)
                                }
                            }
                        });
                        let _ = ws_sender.send(Message::Text(body.to_string().into())).await;
                    }
                }
            }
            true
        }
        Err(broadcast::error::RecvError::Closed) => {
            info!("Broadcast channel closed for {}", client_id);
            false
        }
        Err(broadcast::error::RecvError::Lagged(_)) => {
            warn!("Lagging behind in broadcast channel for {}", client_id);
            true
        }
    }
}
