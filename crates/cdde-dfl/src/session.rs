use std::time::Instant;
use tokio_util::time::delay_queue::Key;

/// Transaction context for session management
#[derive(Debug, Clone)]
pub struct TransactionContext {
    /// DelayQueue key for timeout management
    pub delay_queue_key: Key,
    
    /// Source connection ID (for routing response back)
    pub source_connection_id: u64,
    
    /// Original command code
    pub original_command_code: u32,
    
    /// Original End-to-End ID
    pub original_end_to_end_id: u32,
    
    /// Session ID
    pub session_id: String,
    
    /// Ingress timestamp
    pub ingress_timestamp: Instant,
}

impl TransactionContext {
    /// Create new transaction context
    pub fn new(
        delay_queue_key: Key,
        connection_id: u64,
        command_code: u32,
        end_to_end_id: u32,
        session_id: String,
    ) -> Self {
        Self {
            delay_queue_key,
            source_connection_id: connection_id,
            original_command_code: command_code,
            original_end_to_end_id: end_to_end_id,
            session_id,
            ingress_timestamp: Instant::now(),
        }
    }

    /// Calculate elapsed time since ingress
    pub fn elapsed(&self) -> std::time::Duration {
        self.ingress_timestamp.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::time::DelayQueue;
    use std::time::Duration;

    #[tokio::test]
    async fn test_transaction_context_creation() {
        let mut delay_queue = DelayQueue::new();
        let key = delay_queue.insert((), Duration::from_secs(5));
        
        let ctx = TransactionContext::new(
            key,
            123,
            316,
            999,
            "test-session".to_string(),
        );

        assert_eq!(ctx.source_connection_id, 123);
        assert_eq!(ctx.original_command_code, 316);
        assert_eq!(ctx.original_end_to_end_id, 999);
        assert_eq!(ctx.session_id, "test-session");
    }

    #[tokio::test]
    async fn test_elapsed_time() {
        let mut delay_queue = DelayQueue::new();
        let key = delay_queue.insert((), Duration::from_secs(5));
        
        let ctx = TransactionContext::new(
            key,
            123,
            316,
            999,
            "test-session".to_string(),
        );

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(ctx.elapsed() >= Duration::from_millis(10));
    }
}
