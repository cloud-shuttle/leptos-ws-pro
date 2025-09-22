//! Presence tracking and collaborative features
//!
//! Real-time presence tracking for collaborative applications with automatic cleanup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Presence information for collaborative features
#[derive(Debug, Clone, PartialEq)]
pub struct PresenceMap {
    pub users: HashMap<String, UserPresence>,
    pub last_updated: Instant,
}

impl Default for PresenceMap {
    fn default() -> Self {
        Self {
            users: HashMap::new(),
            last_updated: Instant::now(),
        }
    }
}

impl PresenceMap {
    /// Create a new empty presence map
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a user's presence
    pub fn update_user(&mut self, user_id: String, presence: UserPresence) {
        self.users.insert(user_id, presence);
        self.last_updated = Instant::now();
    }

    /// Remove a user from presence
    pub fn remove_user(&mut self, user_id: &str) -> Option<UserPresence> {
        let result = self.users.remove(user_id);
        if result.is_some() {
            self.last_updated = Instant::now();
        }
        result
    }

    /// Get a user's presence
    pub fn get_user(&self, user_id: &str) -> Option<&UserPresence> {
        self.users.get(user_id)
    }

    /// Get all online users
    pub fn online_users(&self) -> Vec<&UserPresence> {
        self.users
            .values()
            .filter(|presence| presence.is_online())
            .collect()
    }

    /// Get all users with a specific status
    pub fn users_with_status(&self, status: &str) -> Vec<&UserPresence> {
        self.users
            .values()
            .filter(|presence| presence.status == status)
            .collect()
    }

    /// Get the count of online users
    pub fn online_count(&self) -> usize {
        self.users
            .values()
            .filter(|presence| presence.is_online())
            .count()
    }

    /// Clean up stale presence entries (older than timeout)
    pub fn cleanup_stale(&mut self, timeout_ms: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.users.retain(|_, presence| {
            now - presence.last_seen < timeout_ms
        });

        if self.users.is_empty() {
            self.last_updated = Instant::now();
        }
    }

    /// Check if any users are present
    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }

    /// Get all user IDs
    pub fn user_ids(&self) -> Vec<String> {
        self.users.keys().cloned().collect()
    }

    /// Get the length (number of users) - for testing compatibility
    pub fn len(&self) -> usize {
        self.users.len()
    }

    /// Check if contains a key - for testing compatibility
    pub fn contains_key(&self, key: &str) -> bool {
        self.users.contains_key(key)
    }

    /// Index access - for testing compatibility
    pub fn get(&self, key: &str) -> Option<&UserPresence> {
        self.users.get(key)
    }
}

impl std::ops::Index<&str> for PresenceMap {
    type Output = UserPresence;

    fn index(&self, key: &str) -> &Self::Output {
        &self.users[key]
    }
}

/// Individual user presence information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserPresence {
    pub user_id: String,
    pub status: String,
    pub last_seen: u64,
}

impl UserPresence {
    /// Create a new user presence
    pub fn new(user_id: String, status: String) -> Self {
        Self {
            user_id,
            status,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Create an online user presence
    pub fn online(user_id: String) -> Self {
        Self::new(user_id, "online".to_string())
    }

    /// Create an away user presence
    pub fn away(user_id: String) -> Self {
        Self::new(user_id, "away".to_string())
    }

    /// Create an offline user presence
    pub fn offline(user_id: String) -> Self {
        Self::new(user_id, "offline".to_string())
    }

    /// Update the last seen timestamp to now
    pub fn touch(&mut self) {
        self.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// Update the status and touch last seen
    pub fn update_status(&mut self, status: String) {
        self.status = status;
        self.touch();
    }

    /// Check if the user is online
    pub fn is_online(&self) -> bool {
        self.status == "online"
    }

    /// Check if the user is away
    pub fn is_away(&self) -> bool {
        self.status == "away"
    }

    /// Check if the user is offline
    pub fn is_offline(&self) -> bool {
        self.status == "offline"
    }

    /// Get the age of this presence in milliseconds
    pub fn age_ms(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now - self.last_seen
    }

    /// Check if this presence is stale (older than timeout)
    pub fn is_stale(&self, timeout_ms: u64) -> bool {
        self.age_ms() > timeout_ms
    }
}

/// Connection metrics for monitoring
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ConnectionMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connection_uptime: u64,
}

impl ConnectionMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record bytes sent
    pub fn record_bytes_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Record bytes received
    pub fn record_bytes_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    /// Record message sent
    pub fn record_message_sent(&mut self) {
        self.messages_sent += 1;
    }

    /// Record message received
    pub fn record_message_received(&mut self) {
        self.messages_received += 1;
    }

    /// Update connection uptime
    pub fn update_uptime(&mut self, uptime_ms: u64) {
        self.connection_uptime = uptime_ms;
    }

    /// Get total bytes transferred
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }

    /// Get total messages
    pub fn total_messages(&self) -> u64 {
        self.messages_sent + self.messages_received
    }

    /// Reset all metrics to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presence_map_operations() {
        let mut presence_map = PresenceMap::new();
        assert!(presence_map.is_empty());

        let user1 = UserPresence::online("user1".to_string());
        presence_map.update_user("user1".to_string(), user1.clone());

        assert_eq!(presence_map.online_count(), 1);
        assert_eq!(presence_map.get_user("user1"), Some(&user1));
    }

    #[test]
    fn test_user_presence_creation() {
        let user = UserPresence::online("test_user".to_string());
        assert_eq!(user.user_id, "test_user");
        assert_eq!(user.status, "online");
        assert!(user.is_online());
    }

    #[test]
    fn test_connection_metrics() {
        let mut metrics = ConnectionMetrics::new();

        metrics.record_bytes_sent(100);
        metrics.record_message_sent();
        metrics.record_bytes_received(200);
        metrics.record_message_received();

        assert_eq!(metrics.bytes_sent, 100);
        assert_eq!(metrics.bytes_received, 200);
        assert_eq!(metrics.total_bytes(), 300);
        assert_eq!(metrics.total_messages(), 2);
    }

    #[test]
    fn test_presence_cleanup() {
        let mut presence_map = PresenceMap::new();
        let mut user = UserPresence::online("user1".to_string());
        user.last_seen = 0; // Very old timestamp

        presence_map.update_user("user1".to_string(), user);
        assert_eq!(presence_map.online_count(), 1);

        presence_map.cleanup_stale(1000); // 1 second timeout
        assert_eq!(presence_map.online_count(), 0);
    }
}
