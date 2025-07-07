use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use proto::generated::ServerCommand;
use proto::generated::envelope::Payload;
use proto::generated::node_response::Kind;
use proto::generated::request_key::RequestId;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{info, instrument, warn};

use lib_coordinator_core::{AuthState, PendingResponses, ServerRequestByUser};
use proto::generated::{
    Envelope, ServerResponse, ServerStatus, conversation_service_server::ConversationService,
    server_command, server_response,
};

const NODE_CHANNEL_CAPACITY: usize = 1024;
const SERVER_CHANNEL_CAPACITY: usize = 32;

pub struct CoordinatorServiceImpl {
    server_cmd_tx: broadcast::Sender<ServerRequestByUser>,
    nodes: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>,
    start_time: Instant,
    pending: PendingResponses,
}

impl CoordinatorServiceImpl {
    pub fn new(
        nodes: Arc<DashMap<(String, String), broadcast::Sender<Envelope>>>,
        server_cmd_tx: broadcast::Sender<ServerRequestByUser>,
        pending: PendingResponses,
    ) -> Self {
        Self {
            nodes,
            server_cmd_tx,
            start_time: Instant::now(),
            pending,
        }
    }

    fn format_uptime(duration: Duration) -> String {
        let secs = duration.as_secs();
        format!(
            "{}h {:02}m {:02}s",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        )
    }
}

#[tonic::async_trait]
impl ConversationService for CoordinatorServiceImpl {
    type ConversationStream = BoxStream<'static, Result<Envelope, Status>>;

    #[instrument(skip_all)]
    async fn conversation(
        &self,
        request: Request<tonic::Streaming<Envelope>>,
    ) -> Result<Response<Self::ConversationStream>, Status> {
        let auth_state = Arc::new(Mutex::new(AuthState::default()));
        let mut inbound = request.into_inner();
        let (outbound_tx, outbound_rx) = mpsc::channel(SERVER_CHANNEL_CAPACITY);

        let server_cmd_tx = self.server_cmd_tx.clone();
        let nodes = self.nodes.clone();
        let pending = self.pending.clone();
        let start_time = self.start_time;

        // Task 1: Handle server commands -> node
        let server_to_node_handle = {
            let auth_state = auth_state.clone();
            let outbound_tx = outbound_tx.clone();

            tokio::spawn(async move {
                let mut server_cmd_rx = server_cmd_tx.subscribe();
                loop {
                    match server_cmd_rx.recv().await {
                        Ok(request) => {
                            let auth = auth_state.lock().await;
                            if auth.is_match(&request.id, &request.password) {
                                if let Some(Payload::NodeCommand(_)) = &request.envelope.payload {
                                    if let Err(e) = outbound_tx.send(Ok(request.envelope)).await {
                                        warn!("Failed to send server command: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Server command channel error: {}", e);
                            break;
                        }
                    }
                }
                info!("Server->Node task terminated");
            })
        };

        // Task 2: Handle node messages -> server
        let node_to_server_handle = {
            let auth_state = auth_state.clone();
            let outbound_tx = outbound_tx.clone();
            let nodes = nodes.clone();
            let (shutdown_tx, _) = oneshot::channel();

            tokio::spawn(async move {
                let mut shutdown_signal = Some(shutdown_tx);

                while let Some(msg) = inbound.next().await {
                    let envelope = match msg {
                        Ok(e) => e,
                        Err(e) => {
                            warn!("Node stream error: {}", e);
                            break;
                        }
                    };

                    let mut auth = auth_state.lock().await;
                    match envelope.payload {
                        Some(Payload::ServerCommand(cmd)) => {
                            handle_server_command(&mut auth, cmd, &outbound_tx, &nodes, start_time)
                                .await;
                        }
                        Some(Payload::NodeResponse(resp)) => {
                            if auth.is_authenticated() {
                                handle_node_response(resp, &pending, &auth, &nodes).await;
                            }
                        }
                        _ => {}
                    }
                }

                // Cleanup on disconnect
                if let Some((id, password)) = auth_state.lock().await.take_credentials() {
                    nodes.remove(&(id.clone(), password));
                    info!("Node {} disconnected and removed", id);
                }

                if let Some(tx) = shutdown_signal.take() {
                    let _ = tx.send(());
                }
                info!("Node->Server task terminated");
            })
        };

        // Task 3: Cleanup on termination
        tokio::spawn(async move {
            let _ = node_to_server_handle.await;
            server_to_node_handle.abort();
        });

        let stream = ReceiverStream::new(outbound_rx).boxed();
        Ok(Response::new(stream))
    }
}

async fn handle_server_command(
    auth: &mut AuthState,
    cmd: ServerCommand,
    outbound_tx: &mpsc::Sender<Result<Envelope, Status>>,
    nodes: &DashMap<(String, String), broadcast::Sender<Envelope>>,
    start_time: Instant,
) {
    // Handle authentication
    if !auth.is_authenticated() {
        if let Some(server_command::Kind::AuthRequest(auth_req)) = cmd.kind {
            let id = auth_req.node_id;
            let password = auth_req.password;
            auth.authenticate(id.clone(), password.clone());

            // Register new node
            let (tx, _) = broadcast::channel(NODE_CHANNEL_CAPACITY);
            nodes.insert((id, password), tx);
        }
        return;
    }

    // Handle server commands
    if let Some(server_command::Kind::GetServerStatus(_)) = cmd.kind {
        let response = Envelope {
            payload: Some(Payload::ServerResponse(ServerResponse {
                kind: Some(server_response::Kind::ServerStatus(ServerStatus {
                    status: "running".into(),
                    uptime: CoordinatorServiceImpl::format_uptime(start_time.elapsed()),
                })),
            })),
        };

        if let Err(e) = outbound_tx.send(Ok(response)).await {
            warn!("Failed to send server status: {}", e);
        }
    }
}

async fn handle_node_response(
    resp: proto::generated::NodeResponse,
    pending: &PendingResponses,
    auth: &AuthState,
    nodes: &DashMap<(String, String), broadcast::Sender<Envelope>>,
) {
    // Handle pending responses
    if let Some(request_key) = extract_request_key(&resp) {
        if let Some(RequestId::Value(ref id_str)) = request_key.request_id {
            if let Some((_, response_tx)) =
                pending.remove(&(id_str.clone(), request_key.request_type))
            {
                let envelope = Envelope {
                    payload: Some(Payload::NodeResponse(resp)),
                };
                if response_tx.send(envelope).is_err() {
                    warn!(
                        "Pending response channel closed for request {:?}",
                        request_key
                    );
                }
                return;
            }
        }
    }

    info!("Get updates of containers: {:?}", resp);

    // Broadcast to node
    // If it's not the rest request
    if let (Some(id), Some(password)) = (&auth.id, &auth.password) {
        if let Some(node) = nodes.get(&(id.clone(), password.clone())) {
            info!("Get updates of containers: {:?}", resp);

            let envelope = Envelope {
                payload: Some(Payload::NodeResponse(resp)),
            };
            if node.send(envelope).is_err() {
                warn!("Node channel closed for {}", id);
            }
        }
    }
}

fn extract_request_key(
    response: &proto::generated::NodeResponse,
) -> Option<proto::generated::RequestKey> {
    match &response.kind {
        Some(Kind::NodeContainers(c)) => c.request_key.clone(),
        Some(Kind::NodeContainersWithStatus(c)) => c.request_key.clone(),
        Some(Kind::ContainerStatus(c)) => c.request_key.clone(),
        Some(Kind::ContainerLogs(c)) => c.request_key.clone(),
        Some(Kind::ContainerAction(c)) => c.request_key.clone(),
        Some(Kind::Error(c)) => c.request_key.clone(),
        _ => None,
    }
}
