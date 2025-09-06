//! Real-time collaboration primitives for leptos-ws
//!
//! Provides built-in support for collaborative applications using CRDT-inspired
//! approaches with conflict resolution and presence awareness.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

/// Collaborative document with optimistic updates
pub struct CollaborativeDocument<T: Document> {
    /// Local state with optimistic updates
    pub local: RwSignal<T>,
    /// Server-acknowledged state
    pub committed: Signal<T>,
    /// Pending operations queue
    pub operations: VecDeque<Operation>,
    /// Conflict resolution strategy
    pub resolver: Box<dyn ConflictResolver<T>>,
}

/// Trait for documents that can be collaboratively edited
pub trait Document: Clone + Send + Sync + 'static {
    type Operation: Clone + Send + Sync;

    /// Apply an operation to the document
    fn apply(&mut self, operation: &Self::Operation) -> Result<(), ConflictError>;

    /// Get the document's version
    fn version(&self) -> u64;

    /// Set the document's version
    fn set_version(&mut self, version: u64);
}

/// Document operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub document_id: String,
    pub operation_type: OperationType,
    pub timestamp: Instant,
    pub author: String,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationType {
    Insert { position: usize, content: String },
    Delete { position: usize, length: usize },
    Replace { position: usize, old_content: String, new_content: String },
}

/// Conflict resolution strategy
pub trait ConflictResolver<T: Document>: Send + Sync {
    fn resolve(&self, local: &T, remote: &T, operation: &T::Operation) -> Result<T, ConflictError>;
}

/// Conflict resolution errors
#[derive(Debug, thiserror::Error)]
pub enum ConflictError {
    #[error("Operation conflict: {0}")]
    OperationConflict(String),

    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u64, actual: u64 },

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Presence awareness for collaborative editing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresenceAwareness {
    pub user_id: String,
    pub cursor: Option<CursorPosition>,
    pub selection: Option<Selection>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub last_seen: Instant,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
    pub element_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
    pub element_id: Option<String>,
}

/// Fractional indexing for conflict-free ordering
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FractionalIndex {
    value: num_bigint::BigInt,
    client_id: String,
}

impl FractionalIndex {
    pub fn new(client_id: String) -> Self {
        Self {
            value: num_bigint::BigInt::from(0),
            client_id,
        }
    }

    pub fn between(&self, other: &Self) -> Self {
        // Simplified implementation
        Self {
            value: (&self.value + &other.value) / 2,
            client_id: self.client_id.clone(),
        }
    }
}

/// Collaborative text document implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborativeText {
    pub content: String,
    pub version: u64,
    pub operations: Vec<TextOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOperation {
    pub position: usize,
    pub operation_type: TextOperationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextOperationType {
    Insert(String),
    Delete(usize),
}

impl Document for CollaborativeText {
    type Operation = TextOperation;

    fn apply(&mut self, operation: &Self::Operation) -> Result<(), ConflictError> {
        match operation.operation_type {
            TextOperationType::Insert(ref content) => {
                if operation.position > self.content.len() {
                    return Err(ConflictError::InvalidOperation(
                        "Position out of bounds".to_string()
                    ));
                }
                self.content.insert_str(operation.position, content);
            }
            TextOperationType::Delete(length) => {
                if operation.position + length > self.content.len() {
                    return Err(ConflictError::InvalidOperation(
                        "Delete range out of bounds".to_string()
                    ));
                }
                self.content.drain(operation.position..operation.position + length);
            }
        }

        self.operations.push(operation.clone());
        self.version += 1;
        Ok(())
    }

    fn version(&self) -> u64 {
        self.version
    }

    fn set_version(&mut self, version: u64) {
        self.version = version;
    }
}

/// Simple conflict resolver that uses last-write-wins
pub struct LastWriteWinsResolver;

impl<T: Document> ConflictResolver<T> for LastWriteWinsResolver {
    fn resolve(&self, local: &T, _remote: &T, _operation: &T::Operation) -> Result<T, ConflictError> {
        // Simple implementation - just return local state
        Ok(local.clone())
    }
}

/// Hook for collaborative document editing
pub fn use_collaborative_document<T: Document>(
    initial: T,
    document_id: String,
) -> CollaborativeDocument<T> {
    let (local, set_local) = create_signal(initial.clone());
    let (committed, set_committed) = create_signal(initial);
    let operations = VecDeque::new();
    let resolver = Box::new(LastWriteWinsResolver);

    CollaborativeDocument {
        local,
        committed,
        operations,
        resolver,
    }
}

/// Hook for presence awareness
pub fn use_presence_awareness(user_id: String) -> Signal<PresenceAwareness> {
    let (presence, set_presence) = create_signal(PresenceAwareness {
        user_id: user_id.clone(),
        cursor: None,
        selection: None,
        metadata: HashMap::new(),
        last_seen: Instant::now(),
    });

    // Update last_seen periodically
    create_effect(move |_| {
        set_presence.update(|p| {
            p.last_seen = Instant::now();
        });
    });

    presence.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collaborative_text_creation() {
        let text = CollaborativeText {
            content: "Hello, World!".to_string(),
            version: 0,
            operations: Vec::new(),
        };

        assert_eq!(text.content, "Hello, World!");
        assert_eq!(text.version, 0);
    }

    #[test]
    fn test_text_operation_insert() {
        let mut text = CollaborativeText {
            content: "Hello, World!".to_string(),
            version: 0,
            operations: Vec::new(),
        };

        let operation = TextOperation {
            position: 5,
            operation_type: TextOperationType::Insert(", Beautiful".to_string()),
        };

        assert!(text.apply(&operation).is_ok());
        assert_eq!(text.content, "Hello, Beautiful World!");
        assert_eq!(text.version, 1);
    }

    #[test]
    fn test_text_operation_delete() {
        let mut text = CollaborativeText {
            content: "Hello, World!".to_string(),
            version: 0,
            operations: Vec::new(),
        };

        let operation = TextOperation {
            position: 5,
            operation_type: TextOperationType::Delete(2),
        };

        assert!(text.apply(&operation).is_ok());
        assert_eq!(text.content, "Hello World!");
        assert_eq!(text.version, 1);
    }

    #[test]
    fn test_fractional_index_creation() {
        let index = FractionalIndex::new("client1".to_string());
        assert_eq!(index.client_id, "client1");
    }

    #[test]
    fn test_presence_awareness_creation() {
        let presence = PresenceAwareness {
            user_id: "user1".to_string(),
            cursor: Some(CursorPosition { x: 10.0, y: 20.0, element_id: None }),
            selection: None,
            metadata: HashMap::new(),
            last_seen: Instant::now(),
        };

        assert_eq!(presence.user_id, "user1");
        assert!(presence.cursor.is_some());
    }
}
