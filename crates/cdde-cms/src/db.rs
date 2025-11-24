use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use crate::repository::{VirtualRouter, PeerConfig};
use anyhow::Result;

#[derive(Clone)]
pub struct PostgresRepository {
    pool: Pool<Postgres>,
}

impl PostgresRepository {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;

        Ok(Self { pool })
    }

    pub async fn get_all_vrs(&self) -> Vec<VirtualRouter> {
        sqlx::query_as::<_, VirtualRouter>("SELECT id, hostname, realm, timeout_ms FROM virtual_routers")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
    }

    pub async fn get_vr(&self, id: &str) -> Option<VirtualRouter> {
        sqlx::query_as::<_, VirtualRouter>("SELECT id, hostname, realm, timeout_ms FROM virtual_routers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None)
    }

    pub async fn add_vr(&self, vr: VirtualRouter) -> bool {
        sqlx::query(
            "INSERT INTO virtual_routers (id, hostname, realm, timeout_ms) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET hostname = $2, realm = $3, timeout_ms = $4"
        )
        .bind(&vr.id)
        .bind(&vr.hostname)
        .bind(&vr.realm)
        .bind(vr.timeout_ms)
        .execute(&self.pool)
        .await
        .is_ok()
    }

    pub async fn update_vr(&self, vr: VirtualRouter) -> bool {
        sqlx::query(
            "UPDATE virtual_routers SET hostname = $2, realm = $3, timeout_ms = $4 WHERE id = $1"
        )
        .bind(&vr.id)
        .bind(&vr.hostname)
        .bind(&vr.realm)
        .bind(vr.timeout_ms)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected() > 0)
        .unwrap_or(false)
    }

    pub async fn delete_vr(&self, id: &str) -> bool {
        sqlx::query("DELETE FROM virtual_routers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
            .unwrap_or(false)
    }

    pub async fn get_all_peers(&self) -> Vec<PeerConfig> {
        sqlx::query_as::<_, PeerConfig>("SELECT hostname, realm, ip_address, port FROM peers")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
    }

    pub async fn get_peer(&self, hostname: &str) -> Option<PeerConfig> {
        sqlx::query_as::<_, PeerConfig>("SELECT hostname, realm, ip_address, port FROM peers WHERE hostname = $1")
            .bind(hostname)
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None)
    }

    pub async fn add_peer(&self, peer: PeerConfig) -> bool {
        sqlx::query(
            "INSERT INTO peers (hostname, realm, ip_address, port) VALUES ($1, $2, $3, $4) ON CONFLICT (hostname) DO UPDATE SET realm = $2, ip_address = $3, port = $4"
        )
        .bind(&peer.hostname)
        .bind(&peer.realm)
        .bind(&peer.ip_address)
        .bind(peer.port)
        .execute(&self.pool)
        .await
        .is_ok()
    }

    pub async fn delete_peer(&self, hostname: &str) -> bool {
        sqlx::query("DELETE FROM peers WHERE hostname = $1")
            .bind(hostname)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
            .unwrap_or(false)
    }

    // Dictionary management methods
    pub async fn list_dictionaries(&self) -> Vec<crate::models::Dictionary> {
        sqlx::query_as::<_, crate::models::Dictionary>(
            "SELECT id, name, version, xml_content, created_at FROM dictionaries ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_dictionary(&self, id: i32) -> Option<crate::models::Dictionary> {
        sqlx::query_as::<_, crate::models::Dictionary>(
            "SELECT id, name, version, xml_content, created_at FROM dictionaries WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None)
    }

    pub async fn save_dictionary(&self, name: String, version: String, xml_content: String) -> Option<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO dictionaries (name, version, xml_content) VALUES ($1, $2, $3) RETURNING id"
        )
        .bind(&name)
        .bind(&version)
        .bind(&xml_content)
        .fetch_one(&self.pool)
        .await
        .ok()
    }

    pub async fn delete_dictionary(&self, id: i32) -> bool {
        sqlx::query("DELETE FROM dictionaries WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
            .unwrap_or(false)
    }

    // Routing rule management methods
    pub async fn list_routing_rules(&self, vr_id: &str) -> Vec<crate::models::RoutingRule> {
        sqlx::query_as::<_, crate::models::RoutingRule>(
            "SELECT id, vr_id, priority, realm, application_id, destination_host, target_pool, created_at 
             FROM routing_rules WHERE vr_id = $1 ORDER BY priority ASC"
        )
        .bind(vr_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_routing_rule(&self, id: i32) -> Option<crate::models::RoutingRule> {
        sqlx::query_as::<_, crate::models::RoutingRule>(
            "SELECT id, vr_id, priority, realm, application_id, destination_host, target_pool, created_at 
             FROM routing_rules WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None)
    }

    pub async fn create_routing_rule(&self, rule: crate::models::RoutingRule) -> Option<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO routing_rules (vr_id, priority, realm, application_id, destination_host, target_pool) 
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id"
        )
        .bind(&rule.vr_id)
        .bind(rule.priority)
        .bind(&rule.realm)
        .bind(rule.application_id)
        .bind(&rule.destination_host)
        .bind(&rule.target_pool)
        .fetch_one(&self.pool)
        .await
        .ok()
    }

    pub async fn update_routing_rule(&self, rule: crate::models::RoutingRule) -> bool {
        sqlx::query(
            "UPDATE routing_rules 
             SET vr_id = $2, priority = $3, realm = $4, application_id = $5, destination_host = $6, target_pool = $7 
             WHERE id = $1"
        )
        .bind(rule.id)
        .bind(&rule.vr_id)
        .bind(rule.priority)
        .bind(&rule.realm)
        .bind(rule.application_id)
        .bind(&rule.destination_host)
        .bind(&rule.target_pool)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected() > 0)
        .unwrap_or(false)
    }

    pub async fn delete_routing_rule(&self, id: i32) -> bool {
        sqlx::query("DELETE FROM routing_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
            .unwrap_or(false)
    }

    // Manipulation rule management methods
    pub async fn list_manipulation_rules(&self, vr_id: &str) -> Vec<crate::models::ManipulationRule> {
        sqlx::query_as::<_, crate::models::ManipulationRule>(
            "SELECT id, vr_id, priority, rule_json, created_at 
             FROM manipulation_rules WHERE vr_id = $1 ORDER BY priority ASC"
        )
        .bind(vr_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default()
    }

    pub async fn get_manipulation_rule(&self, id: i32) -> Option<crate::models::ManipulationRule> {
        sqlx::query_as::<_, crate::models::ManipulationRule>(
            "SELECT id, vr_id, priority, rule_json, created_at 
             FROM manipulation_rules WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None)
    }

    pub async fn create_manipulation_rule(&self, rule: crate::models::ManipulationRule) -> Option<i32> {
        sqlx::query_scalar::<_, i32>(
            "INSERT INTO manipulation_rules (vr_id, priority, rule_json) 
             VALUES ($1, $2, $3) RETURNING id"
        )
        .bind(&rule.vr_id)
        .bind(rule.priority)
        .bind(&rule.rule_json)
        .fetch_one(&self.pool)
        .await
        .ok()
    }

    pub async fn update_manipulation_rule(&self, rule: crate::models::ManipulationRule) -> bool {
        sqlx::query(
            "UPDATE manipulation_rules 
             SET vr_id = $2, priority = $3, rule_json = $4 
             WHERE id = $1"
        )
        .bind(rule.id)
        .bind(&rule.vr_id)
        .bind(rule.priority)
        .bind(&rule.rule_json)
        .execute(&self.pool)
        .await
        .map(|result| result.rows_affected() > 0)
        .unwrap_or(false)
    }

    pub async fn delete_manipulation_rule(&self, id: i32) -> bool {
        sqlx::query("DELETE FROM manipulation_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected() > 0)
            .unwrap_or(false)
    }
}
