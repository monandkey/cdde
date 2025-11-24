use serde::{Deserialize, Serialize};

/// Virtual Router configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VirtualRouter {
    pub id: String,
    pub hostname: String,
    pub realm: String,
    pub timeout_ms: i32, // Changed to i32 to match DB
}

/// Peer configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PeerConfig {
    pub hostname: String,
    pub realm: String,
    pub ip_address: String,
    pub port: i32, // Changed to i32 to match DB
}
