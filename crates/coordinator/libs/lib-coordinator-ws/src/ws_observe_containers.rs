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
    Envelope, GetNodeContainers, NodeCommand, RequestType, envelope::Payload, node_command,
    node_response::Kind,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast::{
    self,
    error::{self, RecvError},
};
use tracing::{error, info};
use uuid::Uuid;

pub async fn handle_ws_connection(
    Query(auth_params): Query<AuthParams>,
    ws: WebSocketUpgrade,
    Extension(server_tx): Extension<broadcast::Sender<ServerRequestByUser>>,
    Extension(nodes): Extension<Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            auth_params.node_id,
            auth_params.password,
            server_tx,
            nodes,
        )
    })
}

async fn handle_socket(
    socket: WebSocket,
    node_id: String,
    password: String,
    server_tx: broadcast::Sender<ServerRequestByUser>,
    nodes: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    info!("🔌 New WebSocket connection for node: {}", node_id);

    // Check if the node is registered
    let node_key = (node_id.clone(), password.clone());
    let Some(node_tx) = nodes.get(&node_key).map(|g| g.value().clone()) else {
        error!("Node {} not registered", node_id);
        let _ = ws_sender.send(Message::Close(None)).await;
        return;
    };

    // Immediately send a request to get the current containers list
    if let Err(e) = send_get_containers(&server_tx, &node_id, &password).await {
        error!("Failed to send containers request: {}", e);
    }

    // Subscribe to container updates for this node
    let mut broadcast_rx = node_tx.subscribe();
    info!("📡 Containers observing for node: {}", node_id);

    // Main loop: handle both node and server messages
    loop {
        tokio::select! {
            // Handle incoming messages from the WebSocket node
            msg = ws_receiver.next() => {
                if !handle_node_message(msg, &mut ws_sender).await {
                    break;
                }
            }

            // Handle messages from the server (container updates)
            msg = broadcast_rx.recv() => {
                if !handle_server_message(msg, &mut ws_sender, &node_id).await {
                    break;
                }
            }
        }
    }

    info!("🔚 WebSocket session ended for {}", node_id);
}

// Helper to send a GetNodeContainers command to the node
async fn send_get_containers(
    server_tx: &broadcast::Sender<ServerRequestByUser>,
    node_id: &str,
    password: &str,
) -> Result<(), error::SendError<ServerRequestByUser>> {
    server_tx
        .send(ServerRequestByUser {
            id: node_id.to_string(),
            password: password.to_string(),
            envelope: Envelope {
                payload: Some(Payload::NodeCommand(NodeCommand {
                    kind: Some(node_command::Kind::GetNodeContainers(GetNodeContainers {
                        request_id: Uuid::new_v4().to_string(),
                    })),
                })),
            },
        })
        .map(|_| ())
}

// Handle messages from the WebSocket node (pings, closes, etc.)
async fn handle_node_message(
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
            info!("Node disconnected: {:?}", frame);
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
            info!("Node connection closed");
            false
        }
    }
}

// Handle messages from the server (container updates) and send to WebSocket node
async fn handle_server_message(
    msg: Result<Envelope, RecvError>,
    ws_sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    node_id: &str,
) -> bool {
    match msg {
        Ok(envelope) => {
            if let Some(Payload::NodeResponse(resp)) = envelope.payload {
                match resp.kind {
                    Some(Kind::NodeContainers(ref containers_msg)) => {
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
                                    error!("Failed to send to node {}", node_id);
                                    return false;
                                }
                                return true;
                            } else {
                                return true;
                            }
                        } else {
                            return true;
                        }
                    }
                    _ => return true,
                }
            } else {
                return true;
            }
        }
        Err(_e) => {
            error!("Broadcast channel closed for {}", node_id);
            false
        }
    }
}
