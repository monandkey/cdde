use crate::rule::{Rule, Condition, Action, Avp};
use regex::Regex;
use thiserror::Error;

/// Engine error
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(String),
    
    #[error("AVP not found: {0}")]
    AvpNotFound(u32),
}

/// Rule execution engine
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    /// Create new engine with rules
    pub fn new(mut rules: Vec<Rule>) -> Self {
        // Sort by priority (lower number = higher priority)
        rules.sort_by_key(|r| r.priority);
        Self { rules }
    }

    /// Process packet AVPs with rules
    pub fn process(&self, avps: &mut Vec<Avp>) -> Result<(), EngineError> {
        for rule in &self.rules {
            if self.evaluate_conditions(&rule.conditions, avps)? {
                self.execute_actions(&rule.actions, avps)?;
            }
        }
        Ok(())
    }

    /// Evaluate all conditions (AND logic)
    fn evaluate_conditions(&self, conditions: &[Condition], avps: &[Avp]) -> Result<bool, EngineError> {
        for condition in conditions {
            if !self.evaluate_condition(condition, avps)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Evaluate single condition
    fn evaluate_condition(&self, condition: &Condition, avps: &[Avp]) -> Result<bool, EngineError> {
        match condition {
            Condition::AvpExists { code } => {
                Ok(avps.iter().any(|avp| avp.code == *code))
            }
            
            Condition::AvpEquals { code, value } => {
                Ok(avps.iter().any(|avp| avp.code == *code && avp.value == *value))
            }
            
            Condition::AvpMatches { code, pattern } => {
                let regex = Regex::new(pattern)
                    .map_err(|e| EngineError::InvalidRegex(e.to_string()))?;
                
                Ok(avps.iter().any(|avp| {
                    avp.code == *code && regex.is_match(&avp.value)
                }))
            }
            
            Condition::Always => Ok(true),
        }
    }

    /// Execute all actions
    fn execute_actions(&self, actions: &[Action], avps: &mut Vec<Avp>) -> Result<(), EngineError> {
        for action in actions {
            self.execute_action(action, avps)?;
        }
        Ok(())
    }

    /// Execute single action
    fn execute_action(&self, action: &Action, avps: &mut Vec<Avp>) -> Result<(), EngineError> {
        match action {
            Action::AddAvp { code, value } => {
                avps.push(Avp {
                    code: *code,
                    value: value.clone(),
                });
            }
            
            Action::ModifyAvp { code, value } => {
                if let Some(avp) = avps.iter_mut().find(|avp| avp.code == *code) {
                    avp.value = value.clone();
                }
            }
            
            Action::RemoveAvp { code } => {
                avps.retain(|avp| avp.code != *code);
            }
            
            Action::SetAvp { code, value } => {
                if let Some(avp) = avps.iter_mut().find(|avp| avp.code == *code) {
                    avp.value = value.clone();
                } else {
                    avps.push(Avp {
                        code: *code,
                        value: value.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avp_exists_condition() {
        let engine = RuleEngine::new(vec![]);
        let avps = vec![
            Avp { code: 264, value: "test.host".to_string() },
        ];

        let result = engine.evaluate_condition(
            &Condition::AvpExists { code: 264 },
            &avps,
        ).unwrap();

        assert!(result);
    }

    #[test]
    fn test_avp_equals_condition() {
        let engine = RuleEngine::new(vec![]);
        let avps = vec![
            Avp { code: 264, value: "test.host".to_string() },
        ];

        let result = engine.evaluate_condition(
            &Condition::AvpEquals {
                code: 264,
                value: "test.host".to_string(),
            },
            &avps,
        ).unwrap();

        assert!(result);
    }

    #[test]
    fn test_add_avp_action() {
        let engine = RuleEngine::new(vec![]);
        let mut avps = vec![];

        engine.execute_action(
            &Action::AddAvp {
                code: 1,
                value: "user@realm".to_string(),
            },
            &mut avps,
        ).unwrap();

        assert_eq!(avps.len(), 1);
        assert_eq!(avps[0].code, 1);
        assert_eq!(avps[0].value, "user@realm");
    }

    #[test]
    fn test_modify_avp_action() {
        let engine = RuleEngine::new(vec![]);
        let mut avps = vec![
            Avp { code: 264, value: "original.host".to_string() },
        ];

        engine.execute_action(
            &Action::ModifyAvp {
                code: 264,
                value: "modified.host".to_string(),
            },
            &mut avps,
        ).unwrap();

        assert_eq!(avps[0].value, "modified.host");
    }

    #[test]
    fn test_remove_avp_action() {
        let engine = RuleEngine::new(vec![]);
        let mut avps = vec![
            Avp { code: 264, value: "test.host".to_string() },
            Avp { code: 296, value: "test.realm".to_string() },
        ];

        engine.execute_action(
            &Action::RemoveAvp { code: 264 },
            &mut avps,
        ).unwrap();

        assert_eq!(avps.len(), 1);
        assert_eq!(avps[0].code, 296);
    }

    #[test]
    fn test_rule_processing() {
        let rules = vec![
            Rule::new(
                10,
                vec![Condition::AvpExists { code: 264 }],
                vec![Action::AddAvp {
                    code: 1,
                    value: "added@realm".to_string(),
                }],
            ),
        ];

        let engine = RuleEngine::new(rules);
        let mut avps = vec![
            Avp { code: 264, value: "test.host".to_string() },
        ];

        engine.process(&mut avps).unwrap();

        assert_eq!(avps.len(), 2);
        assert_eq!(avps[1].code, 1);
    }
}
