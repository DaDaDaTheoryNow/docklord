use bollard::query_parameters::{EventsOptionsBuilder, ListContainersOptionsBuilder};
use bollard::{Docker, secret::EventMessageTypeEnum};
use futures_util::stream::TryStreamExt;
use proto::generated::request_key::RequestId;
use proto::generated::{
    ClientContainers, ClientResponse, Envelope, client_response, envelope::Payload,
};
use proto::generated::{RequestKey, RequestType};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

pub async fn watch_container_changes(tx: mpsc::Sender<Envelope>) -> Result<(), Box<dyn Error>> {
    let docker = Docker::connect_with_local_defaults()?;
    let mut events_stream = docker.events(Some(EventsOptionsBuilder::default().build()));
    while let Ok(Some(event)) = events_stream.try_next().await {
        if let Some(event_type) = event.typ {
            if event_type == EventMessageTypeEnum::CONTAINER {
                if let Some(action) = event.action {
                    if ["start", "stop", "die", "destroy", "create"].contains(&action.as_str()) {
                        info!(
                            "Container state changed: {} -> {}",
                            event.actor.unwrap_or_default().id.unwrap_or_default(),
                            action
                        );

                        let containers = get_docker_containers().await.unwrap_or_default();

                        let envelope = Envelope {
                            payload: Some(Payload::ClientResponse(ClientResponse {
                                kind: Some(client_response::Kind::ClientContainers(
                                    ClientContainers {
                                        containers,
                                        request_key: Some(RequestKey {
                                            request_type: RequestType::UpdateContainerInfo as i32,
                                            request_id: Some(RequestId::Unspecific(true)),
                                        }),
                                    },
                                )),
                            })),
                        };
                        if tx.send(envelope).await.is_err() {
                            error!("Failed to send container change message");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn get_docker_containers() -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;
    let containers = docker
        .list_containers(Some(
            ListContainersOptionsBuilder::default().all(true).build(),
        ))
        .await?;
    let container_names: Vec<String> = containers
        .into_iter()
        .filter_map(|container| {
            container.names.and_then(|names| {
                names
                    .first()
                    .map(|name| name.trim_start_matches('/').to_string())
            })
        })
        .collect();
    Ok(container_names)
}
