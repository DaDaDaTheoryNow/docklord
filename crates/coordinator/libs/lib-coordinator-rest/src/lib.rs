mod get_containers;
mod rest_server;

pub use rest_server::build_rest_router;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthParams {
    pub node_id: String,
    pub password: String,
}
