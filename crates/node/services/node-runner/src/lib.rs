pub async fn run(
    coordinator_address: &str,
    node_id: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "==============================\n\
🚀 Node started!\n\
Use these credentials to connect via Coordinator ({}):\n\
  node_id:   {}\n\
  password:  {}\n\
==============================",
        coordinator_address, node_id, password
    );

    println!("");

    lib_node_grpc::run_grpc_client(coordinator_address, node_id, password).await
}
