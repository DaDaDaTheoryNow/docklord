pub mod container_actions;
pub mod container_logs;
pub mod container_status;
pub mod get_containers;
pub mod rest_server;

pub use rest_server::build_rest_router;

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthParams {
    pub node_id: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
struct ApiErrorDetail {
    message: String,
    detail: String,
}

#[derive(Deserialize, Serialize)]
struct ApiError {
    req_uuid: String,
    error: ApiErrorDetail,
}
