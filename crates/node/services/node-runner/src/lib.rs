pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client_id = "client1";
    let password = "secret123";

    println!(
        "==============================\n\
🚀 Node started!\n\
Use these credentials to connect via Coordinator:\n\
  client_id:   {}\n\
  password:  {}\n\
==============================",
        client_id, password
    );

    println!("");

    lib_node_grpc::run_grpc_client("http://[::1]:50051", client_id, password).await
}
