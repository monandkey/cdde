use serde::{Deserialize, Serialize};

/// Virtual Router configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualRouter {
    pub id: String,
    pub hostname: String,
    pub realm: String,
    pub timeout_ms: u64,
}

/// Peer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub peer_id: String,
    pub host_name: String,
    pub realm: String,
    pub ip_addresses: Vec<String>,
}

/// Configuration repository (in-memory for now)
pub struct ConfigRepository {
    virtual_routers: std::sync::Arc<tokio::sync::RwLock<Vec<VirtualRouter>>>,
    peers: std::sync::Arc<tokio::sync::RwLock<Vec<PeerConfig>>>,
}

impl ConfigRepository {
    /// Create new repository
    pub fn new() -> Self {
        Self {
            virtual_routers: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            peers: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Add virtual router
    pub async fn add_vr(&self, vr: VirtualRouter) {
        let mut vrs = self.virtual_routers.write().await;
        vrs.push(vr);
    }

    /// Get all virtual routers
    pub async fn get_all_vrs(&self) -> Vec<VirtualRouter> {
        let vrs = self.virtual_routers.read().await;
        vrs.clone()
    }

    /// Get virtual router by ID
    pub async fn get_vr(&self, id: &str) -> Option<VirtualRouter> {
        let vrs = self.virtual_routers.read().await;
        vrs.iter().find(|vr| vr.id == id).cloned()
    }

    /// Delete virtual router
    pub async fn delete_vr(&self, id: &str) -> bool {
        let mut vrs = self.virtual_routers.write().await;
        if let Some(pos) = vrs.iter().position(|vr| vr.id == id) {
            vrs.remove(pos);
            true
        } else {
            false
        }
    }

    /// Add peer
    pub async fn add_peer(&self, peer: PeerConfig) {
        let mut peers = self.peers.write().await;
        peers.push(peer);
    }

    /// Get all peers
    pub async fn get_all_peers(&self) -> Vec<PeerConfig> {
        let peers = self.peers.read().await;
        peers.clone()
    }
}

impl Default for ConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_vr() {
        let repo = ConfigRepository::new();
        
        let vr = VirtualRouter {
            id: "vr001".to_string(),
            hostname: "dcr-vr001.test".to_string(),
            realm: "test.realm".to_string(),
            timeout_ms: 5000,
        };

        repo.add_vr(vr.clone()).await;
        
        let retrieved = repo.get_vr("vr001").await.unwrap();
        assert_eq!(retrieved.id, "vr001");
        assert_eq!(retrieved.hostname, "dcr-vr001.test");
    }

    #[tokio::test]
    async fn test_get_all_vrs() {
        let repo = ConfigRepository::new();
        
        repo.add_vr(VirtualRouter {
            id: "vr001".to_string(),
            hostname: "dcr-vr001.test".to_string(),
            realm: "test.realm".to_string(),
            timeout_ms: 5000,
        }).await;

        repo.add_vr(VirtualRouter {
            id: "vr002".to_string(),
            hostname: "dcr-vr002.test".to_string(),
            realm: "test.realm".to_string(),
            timeout_ms: 5000,
        }).await;

        let vrs = repo.get_all_vrs().await;
        assert_eq!(vrs.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_vr() {
        let repo = ConfigRepository::new();
        
        repo.add_vr(VirtualRouter {
            id: "vr001".to_string(),
            hostname: "dcr-vr001.test".to_string(),
            realm: "test.realm".to_string(),
            timeout_ms: 5000,
        }).await;

        assert!(repo.delete_vr("vr001").await);
        assert!(repo.get_vr("vr001").await.is_none());
    }

    #[tokio::test]
    async fn test_add_and_get_peers() {
        let repo = ConfigRepository::new();
        
        repo.add_peer(PeerConfig {
            peer_id: "peer01".to_string(),
            host_name: "hss01.operator.net".to_string(),
            realm: "operator.net".to_string(),
            ip_addresses: vec!["10.0.1.10".to_string()],
        }).await;

        let peers = repo.get_all_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer01");
    }
}
