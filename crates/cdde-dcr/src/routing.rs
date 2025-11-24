use serde::{Deserialize, Serialize};

/// Routing decision result
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Target peer hostname
    pub target_peer: String,
    
    /// Routing priority
    pub priority: u8,
}

/// Route entry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub priority: u8,
    pub condition: RouteCondition,
    pub target_pool_id: String,
}

/// Routing condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RouteCondition {
    DestinationHost { value: String },
    ApplicationCommand { app_id: u32, command_code: u32 },
    DestinationRealm { value: String },
    Default,
}

/// Simple routing engine
pub struct RoutingEngine {
    routes: Vec<RouteEntry>,
}

impl RoutingEngine {
    /// Create new routing engine with routes
    pub fn new(routes: Vec<RouteEntry>) -> Self {
        let mut sorted_routes = routes;
        sorted_routes.sort_by_key(|r| r.priority);
        Self { routes: sorted_routes }
    }

    /// Find route for given parameters
    pub fn find_route(
        &self,
        dest_host: Option<&str>,
        dest_realm: Option<&str>,
        app_id: u32,
        command_code: u32,
    ) -> Option<RoutingDecision> {
        for route in &self.routes {
            if self.matches(&route.condition, dest_host, dest_realm, app_id, command_code) {
                // For now, use pool_id as target peer
                return Some(RoutingDecision {
                    target_peer: route.target_pool_id.clone(),
                    priority: route.priority,
                });
            }
        }
        None
    }

    fn matches(
        &self,
        condition: &RouteCondition,
        dest_host: Option<&str>,
        dest_realm: Option<&str>,
        app_id: u32,
        command_code: u32,
    ) -> bool {
        match condition {
            RouteCondition::DestinationHost { value } => {
                dest_host.map_or(false, |h| h == value)
            }
            RouteCondition::ApplicationCommand { app_id: a, command_code: c } => {
                *a == app_id && *c == command_code
            }
            RouteCondition::DestinationRealm { value } => {
                dest_realm.map_or(false, |r| r == value)
            }
            RouteCondition::Default => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destination_host_routing() {
        let routes = vec![
            RouteEntry {
                priority: 10,
                condition: RouteCondition::DestinationHost {
                    value: "hss01.operator.net".to_string(),
                },
                target_pool_id: "pool-hss-primary".to_string(),
            },
        ];

        let engine = RoutingEngine::new(routes);
        let decision = engine.find_route(
            Some("hss01.operator.net"),
            None,
            0,
            0,
        ).unwrap();

        assert_eq!(decision.target_peer, "pool-hss-primary");
        assert_eq!(decision.priority, 10);
    }

    #[test]
    fn test_application_command_routing() {
        let routes = vec![
            RouteEntry {
                priority: 20,
                condition: RouteCondition::ApplicationCommand {
                    app_id: 16777251,
                    command_code: 316,
                },
                target_pool_id: "pool-hss-s6a".to_string(),
            },
        ];

        let engine = RoutingEngine::new(routes);
        let decision = engine.find_route(
            None,
            None,
            16777251,
            316,
        ).unwrap();

        assert_eq!(decision.target_peer, "pool-hss-s6a");
    }

    #[test]
    fn test_default_routing() {
        let routes = vec![
            RouteEntry {
                priority: 100,
                condition: RouteCondition::Default,
                target_pool_id: "pool-default".to_string(),
            },
        ];

        let engine = RoutingEngine::new(routes);
        let decision = engine.find_route(
            Some("unknown.host"),
            None,
            999,
            999,
        ).unwrap();

        assert_eq!(decision.target_peer, "pool-default");
    }

    #[test]
    fn test_no_route_found() {
        let routes = vec![
            RouteEntry {
                priority: 10,
                condition: RouteCondition::DestinationHost {
                    value: "specific.host".to_string(),
                },
                target_pool_id: "pool-specific".to_string(),
            },
        ];

        let engine = RoutingEngine::new(routes);
        let decision = engine.find_route(
            Some("other.host"),
            None,
            0,
            0,
        );

        assert!(decision.is_none());
    }
}
