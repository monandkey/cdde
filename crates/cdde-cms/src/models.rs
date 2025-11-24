use serde::{Deserialize, Serialize};

/// Virtual Router configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VirtualRouter {
    pub id: String,
    pub hostname: String,
    pub realm: String,
    pub timeout_ms: i32,
}

/// Peer configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PeerConfig {
    pub hostname: String,
    pub realm: String,
    pub ip_address: String,
    pub port: i32,
}

/// Dictionary metadata
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Dictionary {
    pub id: i32,
    pub name: String,
    pub version: String,
    pub xml_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Dictionary AVP definition
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DictionaryAvp {
    pub id: i32,
    pub dictionary_id: i32,
    pub code: i32,
    pub name: String,
    pub data_type: String,
    pub vendor_id: Option<i32>,
}

/// Routing rule configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoutingRule {
    pub id: i32,
    pub vr_id: String,
    pub priority: i32,
    pub realm: Option<String>,
    pub application_id: Option<i32>,
    pub destination_host: Option<String>,
    pub target_pool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Manipulation rule (DSL)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ManipulationRule {
    pub id: i32,
    pub vr_id: String,
    pub priority: i32,
    pub rule_json: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
