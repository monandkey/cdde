use dashmap::DashMap;
use std::sync::Arc;
use tokio_util::time::{DelayQueue, delay_queue::Key};
use std::time::Duration;

use crate::session::TransactionContext;

/// Transaction store using DashMap for concurrent access
pub struct TransactionStore {
    /// Map of (ConnectionID, Hop-by-Hop ID) -> TransactionContext
    store: Arc<DashMap<(u64, u32), TransactionContext>>,
    
    /// Delay queue for timeout management
    delay_queue: tokio::sync::Mutex<DelayQueue<(u64, u32)>>,
}

impl TransactionStore {
    /// Create new transaction store
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
            delay_queue: tokio::sync::Mutex::new(DelayQueue::new()),
        }
    }

    /// Insert new transaction with timeout
    pub async fn insert(
        &self,
        connection_id: u64,
        hop_by_hop_id: u32,
        command_code: u32,
        end_to_end_id: u32,
        session_id: String,
        timeout: Duration,
    ) -> Key {
        let key = (connection_id, hop_by_hop_id);
        
        // Add to delay queue
        let mut delay_queue = self.delay_queue.lock().await;
        let delay_key = delay_queue.insert(key, timeout);
        drop(delay_queue);

        // Create context
        let context = TransactionContext::new(
            delay_key,
            connection_id,
            command_code,
            end_to_end_id,
            session_id,
        );

        // Store in map
        self.store.insert(key, context);
        
        delay_key
    }

    /// Remove transaction and cancel timeout
    pub async fn remove(&self, connection_id: u64, hop_by_hop_id: u32) -> Option<TransactionContext> {
        let key = (connection_id, hop_by_hop_id);
        
        if let Some((_, context)) = self.store.remove(&key) {
            // Cancel timeout
            let mut delay_queue = self.delay_queue.lock().await;
            delay_queue.remove(&context.delay_queue_key);
            
            Some(context)
        } else {
            None
        }
    }

    /// Get transaction without removing
    pub fn get(&self, connection_id: u64, hop_by_hop_id: u32) -> Option<TransactionContext> {
        let key = (connection_id, hop_by_hop_id);
        self.store.get(&key).map(|entry| entry.clone())
    }

    /// Get number of active transactions
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Wait for next timeout
    pub async fn next_timeout(&self) -> Option<(u64, u32)> {
        use futures::StreamExt;
        let mut delay_queue = self.delay_queue.lock().await;
        delay_queue.next().await.map(|expired| expired.into_inner())
    }
}

impl Default for TransactionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_get() {
        let store = TransactionStore::new();
        
        store.insert(
            123,
            456,
            316,
            999,
            "test-session".to_string(),
            Duration::from_secs(5),
        ).await;

        let context = store.get(123, 456).unwrap();
        assert_eq!(context.source_connection_id, 123);
        assert_eq!(context.session_id, "test-session");
    }

    #[tokio::test]
    async fn test_remove() {
        let store = TransactionStore::new();
        
        store.insert(
            123,
            456,
            316,
            999,
            "test-session".to_string(),
            Duration::from_secs(5),
        ).await;

        assert_eq!(store.len(), 1);

        let context = store.remove(123, 456).await.unwrap();
        assert_eq!(context.source_connection_id, 123);
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn test_timeout() {
        let store = TransactionStore::new();
        
        store.insert(
            123,
            456,
            316,
            999,
            "test-session".to_string(),
            Duration::from_millis(100),
        ).await;

        // Wait for timeout
        let expired = store.next_timeout().await.unwrap();
        assert_eq!(expired, (123, 456));
    }
}
