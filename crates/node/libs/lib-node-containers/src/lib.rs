// --- Container management logic for Docklord ---
// The following code was written by an AI assistant (GPT-4) at the user's request.
// It implements REST/gRPC handlers for container status, start/stop/delete, and logs with detailed options.

use bollard::query_parameters::{
    EventsOptionsBuilder, ListContainersOptionsBuilder, LogsOptionsBuilder,
    RemoveContainerOptionsBuilder, StartContainerOptionsBuilder, StopContainerOptionsBuilder,
};
use bollard::{Docker, secret::EventMessageTypeEnum};
use chrono;
use futures_util::stream::TryStreamExt;
use proto::generated::request_key::RequestId;
use proto::generated::{Envelope, envelope::Payload};
use proto::generated::{NodeContainers, NodeResponse, RequestKey, RequestType, node_response};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Watches for Docker container events and notifies the system about changes.
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
                            payload: Some(Payload::NodeResponse(NodeResponse {
                                kind: Some(node_response::Kind::NodeContainers(NodeContainers {
                                    containers,
                                    request_key: Some(RequestKey {
                                        request_type: RequestType::UpdateContainerInfo as i32,
                                        request_id: Some(RequestId::Unspecific(true)),
                                    }),
                                })),
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

/// Returns a list of all Docker containers (by name).
/// Used for the REST endpoint /api/containers
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

/// Returns detailed status for a specific container.
/// Used for /api/containers/:container_id/status
pub async fn get_container_status(
    container_id: &str,
) -> Result<proto::generated::ContainerStatus, Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;
    let container_info = docker
        .inspect_container(
            container_id,
            Some(bollard::query_parameters::InspectContainerOptionsBuilder::default().build()),
        )
        .await?;

    let state = container_info.state.unwrap_or_default();
    let status = state
        .status
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Parse timestamps properly
    let created = container_info
        .created
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    let started_at = state
        .started_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    let finished_at = state
        .finished_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    let exit_code = state.exit_code.unwrap_or(0).try_into().unwrap_or(0);

    Ok(proto::generated::ContainerStatus {
        request_key: None, // will be set by the handler
        container_id: container_id.to_string(),
        status,
        created,
        started_at,
        finished_at,
        exit_code,
    })
}

/// Starts a container by id. Used for /api/containers/:container_id/start
pub async fn start_container(
    container_id: &str,
) -> Result<proto::generated::ContainerAction, Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;

    match docker
        .start_container(
            container_id,
            Some(StartContainerOptionsBuilder::default().build()),
        )
        .await
    {
        Ok(_) => Ok(proto::generated::ContainerAction {
            request_key: None, // будет установлено в обработчике
            container_id: container_id.to_string(),
            action: "start".to_string(),
            message: "Container started successfully".to_string(),
        }),
        Err(e) => Err(e.into()),
    }
}

/// Stops a container by id. Used for /api/containers/:container_id/stop
pub async fn stop_container(
    container_id: &str,
) -> Result<proto::generated::ContainerAction, Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;

    match docker
        .stop_container(
            container_id,
            Some(StopContainerOptionsBuilder::default().build()),
        )
        .await
    {
        Ok(_) => Ok(proto::generated::ContainerAction {
            request_key: None, // будет установлено в обработчике
            container_id: container_id.to_string(),
            action: "stop".to_string(),
            message: "Container stopped successfully".to_string(),
        }),
        Err(e) => Err(e.into()),
    }
}

/// Deletes a container by id. Used for DELETE /api/containers/:container_id
pub async fn delete_container(
    container_id: &str,
) -> Result<proto::generated::ContainerAction, Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;

    match docker
        .remove_container(
            container_id,
            Some(RemoveContainerOptionsBuilder::default().build()),
        )
        .await
    {
        Ok(_) => Ok(proto::generated::ContainerAction {
            request_key: None, // будет установлено в обработчике
            container_id: container_id.to_string(),
            action: "delete".to_string(),
            message: "Container deleted successfully".to_string(),
        }),
        Err(e) => Err(e.into()),
    }
}

/// Returns logs for a container. Supports tail, follow, since options.
/// Used for /api/containers/:container_id/logs
pub async fn get_container_logs(
    container_id: &str,
    tail: Option<i32>,
    follow: bool,
    since: Option<String>,
) -> Result<proto::generated::ContainerLogs, Box<dyn Error + Send + Sync>> {
    let docker = Docker::connect_with_local_defaults()?;

    let mut logs_builder = LogsOptionsBuilder::default();
    logs_builder = logs_builder.stdout(true);
    logs_builder = logs_builder.stderr(true);
    if let Some(t) = tail {
        logs_builder = logs_builder.tail(&t.to_string());
    }
    logs_builder = logs_builder.follow(follow);
    if let Some(s) = since {
        if let Ok(timestamp) = s.parse::<i64>() {
            logs_builder = logs_builder.since(timestamp.try_into().unwrap());
        }
    }

    let options = logs_builder.build();
    let mut stream = docker.logs(container_id, Some(options));

    let mut logs = Vec::new();

    // Если follow = false, читаем все доступные логи
    if !follow {
        while let Ok(Some(log)) = stream.try_next().await {
            if let Ok(log_line) = String::from_utf8(log.into_bytes().to_vec()) {
                logs.push(log_line);
            }
        }
    } else {
        // Для follow = true читаем только последние логи
        let mut count = 0;
        while let Ok(Some(log)) = stream.try_next().await {
            if let Ok(log_line) = String::from_utf8(log.into_bytes().to_vec()) {
                logs.push(log_line);
                count += 1;
                if count >= tail.unwrap_or(100) {
                    break;
                }
            }
        }
    }

    Ok(proto::generated::ContainerLogs {
        request_key: None, // будет установлено в обработчике
        container_id: container_id.to_string(),
        logs,
    })
}
