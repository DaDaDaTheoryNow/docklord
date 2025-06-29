use proto::generated::conversation_service_server::ConversationServiceServer;
use tonic::transport::Server;

use crate::grpc_server_service::CoordinatorServiceImpl;

pub async fn run_grpc_server(
    coordinator_service: CoordinatorServiceImpl,
    grpc_coordinator_addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Server::builder()
        .add_service(ConversationServiceServer::new(coordinator_service))
        .serve(grpc_coordinator_addr)
        .await?;
    Ok(())
}
