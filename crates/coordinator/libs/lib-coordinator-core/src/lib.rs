pub mod auth_state;

use std::sync::Arc;

use dashmap::DashMap;
use proto::generated::Envelope;
use tokio::sync::oneshot;

pub use auth_state::AuthState;

pub type PendingResponses = Arc<DashMap<(String, i32), oneshot::Sender<Envelope>>>;

#[derive(Debug, Clone, PartialEq)]
pub struct ServerRequestByUser {
    pub envelope: Envelope,
    pub id: String,
    pub password: String,
}
