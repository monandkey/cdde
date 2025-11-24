use serde::{Deserialize, Serialize};
use validator::Validate;
use utoipa::ToSchema;

/// Virtual Router configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct VirtualRouter {
    #[validate(length(min = 1, message = "ID cannot be empty"))]
    #[schema(example = "vr1")]
    pub id: String,
    
    #[validate(length(min = 1, message = "Hostname cannot be empty"))]
    #[schema(example = "host1.example.com")]
    pub hostname: String,
    
    #[validate(length(min = 1, message = "Realm cannot be empty"))]
    #[schema(example = "example.com")]
    pub realm: String,
    
    #[validate(range(min = 100, message = "Timeout must be at least 100ms"))]
    #[schema(example = 3000)]
    pub timeout_ms: i32,
}

/// Peer configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct PeerConfig {
    #[validate(length(min = 1, message = "Hostname cannot be empty"))]
    #[schema(example = "peer1.example.com")]
    pub hostname: String,
    
    #[validate(length(min = 1, message = "Realm cannot be empty"))]
    #[schema(example = "example.com")]
    pub realm: String,
    
    #[validate(length(min = 1, message = "IP address cannot be empty"))] // Could add IP validation regex
    #[schema(example = "192.168.1.10")]
    pub ip_address: String,
    
    #[validate(range(min = 1, max = 65535, message = "Port must be between 1 and 65535"))]
    #[schema(example = 3868)]
    pub port: i32,
}

/// Dictionary metadata
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Dictionary {
    #[schema(example = 1)]
    pub id: i32,
    #[schema(example = "base_dictionary")]
    pub name: String,
    #[schema(example = "1.0")]
    pub version: String,
    #[schema(example = "<dictionary>...</dictionary>")]
    pub xml_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Dictionary AVP definition
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct DictionaryAvp {
    pub id: i32,
    pub dictionary_id: i32,
    pub code: i32,
    pub name: String,
    pub data_type: String,
    pub vendor_id: Option<i32>,
}

/// Routing rule configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct RoutingRule {
    #[serde(default)] // Allow omitting ID for creation
    pub id: i32,
    
    #[validate(length(min = 1, message = "VR ID cannot be empty"))]
    #[schema(example = "vr1")]
    pub vr_id: String,
    
    #[schema(example = 10)]
    pub priority: i32,
    
    #[schema(example = "example.com")]
    pub realm: Option<String>,
    
    #[schema(example = 16777251)]
    pub application_id: Option<i32>,
    
    #[schema(example = "dest.example.com")]
    pub destination_host: Option<String>,
    
    #[validate(length(min = 1, message = "Target pool cannot be empty"))]
    #[schema(example = "pool1")]
    pub target_pool: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Manipulation rule (DSL)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Validate, ToSchema)]
pub struct ManipulationRule {
    #[serde(default)]
    pub id: i32,
    
    #[validate(length(min = 1, message = "VR ID cannot be empty"))]
    #[schema(example = "vr1")]
    pub vr_id: String,
    
    #[schema(example = 10)]
    pub priority: i32,
    
    // serde_json::Value doesn't implement ToSchema automatically, usually needs manual handling or raw type
    #[schema(value_type = Object)] 
    pub rule_json: serde_json::Value,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
