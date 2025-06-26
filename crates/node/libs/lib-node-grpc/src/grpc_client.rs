use std::error::Error;

use futures_util::StreamExt;
use lib_node_containers::{get_docker_containers, watch_container_changes};
use proto::generated::{
    AuthRequest, ClientContainers, ClientResponse, Envelope, RequestKey, RequestType,
    ServerCommand, client_command, client_response,
    conversation_service_client::ConversationServiceClient, envelope::Payload,
    request_key::RequestId, server_command, server_response,
};
use tokio::sync::{mpsc, oneshot};
use tokio_stream;
use tonic::transport::Channel;
use tracing::{error, info};

pub async fn run_grpc_client(
    address: &str,
    client_id: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address_owned = address.to_string();
    let channel = Channel::from_static(Box::leak(address_owned.into_boxed_str()))
        .connect()
        .await?;
    let mut client = ConversationServiceClient::new(channel);

    let (tx_out, rx_out) = mpsc::channel(100);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let request = tonic::Request::new(tokio_stream::wrappers::ReceiverStream::new(rx_out));
    let mut stream = client.conversation(request).await?.into_inner();

    let auth_envelope = Envelope {
        payload: Some(Payload::ServerCommand(ServerCommand {
            kind: Some(server_command::Kind::AuthRequest(AuthRequest {
                client_id: client_id.into(),
                password: password.into(),
            })),
        })),
    };

    let status_envelope = Envelope {
        payload: Some(Payload::ServerCommand(ServerCommand {
            kind: Some(server_command::Kind::GetServerStatus(Default::default())),
        })),
    };

    tx_out.send(auth_envelope).await?;
    tx_out.send(status_envelope).await?;

    let tx_clone_for_docker = tx_out.clone();
    tokio::spawn(async move {
        if let Err(e) = watch_container_changes(tx_clone_for_docker).await {
            let err_str = e.to_string();
            if err_str.contains("Socket not found: /var/run/docker.sock") {
                error!("Docker socket not found. Docker is probably not running.");
            } else {
                error!("Error watching containers: {}", err_str);
            }
        }
    });

    let tx_clone = tx_out.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                maybe_msg = stream.next() => {
                    match maybe_msg {
                        Some(Ok(envelope)) => {
                            if let Err(e) = process_incoming_message(envelope, &tx_clone).await {
                                error!("Error processing message: {}", e);
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("Stream error: {}", e);
                            break;
                        }
                        None => {
                            info!("Stream closed by server");
                            break;
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }
    });

    info!("Client started. Press Ctrl+C to exit.");
    tokio::signal::ctrl_c().await?;
    let _ = shutdown_tx.send(());

    info!("Client stopped");
    Ok(())
}

pub async fn handle_get_client_containers(
    tx: &mpsc::Sender<Envelope>,
    request_id: String,
) -> Result<(), String> {
    let containers = get_docker_containers().await.unwrap_or_default();
    let response = Envelope {
        payload: Some(Payload::ClientResponse(ClientResponse {
            kind: Some(client_response::Kind::ClientContainers(ClientContainers {
                request_key: Some(RequestKey {
                    request_type: RequestType::GetContainers as i32,
                    request_id: Some(RequestId::Value(request_id)),
                }),
                containers: containers,
            })),
        })),
    };

    tx.send(response)
        .await
        .map_err(|_| String::from("Failed to send response"))?;

    Ok(())
}

pub async fn process_incoming_message(
    envelope: Envelope,
    tx: &mpsc::Sender<Envelope>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match envelope.payload {
        Some(Payload::ClientCommand(cmd)) => match cmd.kind {
            Some(client_command::Kind::GetClientContainers(get_containers_request)) => {
                handle_get_client_containers(tx, get_containers_request.request_id).await?;
            }
            _ => info!("Unknown client command"),
        },
        Some(Payload::ServerResponse(resp)) => {
            if let Some(server_response::Kind::ServerStatus(status)) = &resp.kind {
                info!(
                    "Server status: {}, uptime: {}",
                    status.status, status.uptime
                );
            }
            if let Some(server_response::Kind::AuthResponse(response)) = &resp.kind {
                info!(
                    "Auth result: {}, message: {}",
                    response.success, response.message
                );
            }
        }
        _ => info!("Received unknown message"),
    }
    Ok(())
}
