pub async fn run(
    coordinator_address: &str,
    node_id: &str,
    password: &str,
    is_self_hosted: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if is_self_hosted {
        println!(
            "==============================\n\
🚀 Node started\n\
Coordinator gRPC address (set as COORDINATOR_ADDR in docklord-node mode; not a browser URL):\n\
  {0}\n\
Credentials for Coordinator-authenticated requests:\n\
  node_id:   {1}\n\
  password:  {2}\n\
Example:\n\
  curl \"http://localhost:3000/api/containers?node_id={1}&password={2}\"\n\
==============================",
            coordinator_address, node_id, password
        );
    } else {
        // Убираем порт, если есть
        let host_only = coordinator_address
            .split("://")
            .last()
            .unwrap_or(coordinator_address) // убираем протокол, если он есть
            .split(':')
            .next()
            .unwrap_or(coordinator_address); // убираем порт

        // Replace docklord-coordinator with localhost in the example URL
        let example_host = if host_only == "docklord-coordinator" {
            "localhost"
        } else {
            host_only
        };

        println!(
            "==============================\n\
🚀 Node started\n\
Coordinator gRPC address (set as COORDINATOR_ADDR in docklord-node mode; not a browser URL):\n\
  {0}\n\
Credentials for Coordinator-authenticated requests:\n\
  node_id:   {1}\n\
  password:  {2}\n\
Example:\n\
  curl \"http://{3}:3000/api/containers?node_id={1}&password={2}\"\n\
==============================",
            coordinator_address, node_id, password, example_host
        );
    }

    println!();

    lib_node_grpc::run_grpc_client(coordinator_address, node_id, password).await
}
