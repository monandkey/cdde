use serde::{Deserialize, Serialize};

/// Manipulation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub priority: u8,
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

/// Condition for rule matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// Check if AVP exists
    AvpExists { code: u32 },
    
    /// Check if AVP equals specific value
    AvpEquals { code: u32, value: String },
    
    /// Check if AVP matches regex pattern
    AvpMatches { code: u32, pattern: String },
    
    /// Always true (default condition)
    Always,
}

/// Action to perform on packet
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    /// Add new AVP
    AddAvp {
        code: u32,
        value: String,
    },
    
    /// Modify existing AVP
    ModifyAvp {
        code: u32,
        value: String,
    },
    
    /// Remove AVP
    RemoveAvp {
        code: u32,
    },
    
    /// Set AVP (add if not exists, modify if exists)
    SetAvp {
        code: u32,
        value: String,
    },
}

/// AVP representation for manipulation
#[derive(Debug, Clone)]
pub struct Avp {
    pub code: u32,
    pub value: String,
}

impl Rule {
    /// Create new rule
    pub fn new(priority: u8, conditions: Vec<Condition>, actions: Vec<Action>) -> Self {
        Self {
            priority,
            conditions,
            actions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_creation() {
        let rule = Rule::new(
            10,
            vec![Condition::AvpExists { code: 264 }],
            vec![Action::AddAvp {
                code: 1,
                value: "test@realm".to_string(),
            }],
        );

        assert_eq!(rule.priority, 10);
        assert_eq!(rule.conditions.len(), 1);
        assert_eq!(rule.actions.len(), 1);
    }

    #[test]
    fn test_rule_serialization() {
        let rule = Rule::new(
            10,
            vec![Condition::AvpEquals {
                code: 264,
                value: "test.host".to_string(),
            }],
            vec![Action::ModifyAvp {
                code: 264,
                value: "modified.host".to_string(),
            }],
        );

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: Rule = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.priority, 10);
    }
}
