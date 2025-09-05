use std::borrow::Cow;

use json_patch::Patch;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Messages {
    ServerSignal(ServerSignalMessage),
    // Hier können weitere Nachrichtentypen hinzugefügt werden
    // ChatMessage(ChatMessage),
    // StateSync(StateSyncMessage),
    // etc.
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ServerSignalMessage {
    Establish(String),
    EstablishResponse((String, Value)),
    Update(ServerSignalUpdate),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSignalUpdate {
    pub name: Cow<'static, str>,
    pub patch: Patch,
}

impl ServerSignalUpdate {
    /// Creates a new [`ServerSignalUpdate`] from an old and new instance of `T`.
    pub fn new<T>(
        name: impl Into<Cow<'static, str>>,
        old: &T,
        new: &T,
    ) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        let left = serde_json::to_value(old)?;
        let right = serde_json::to_value(new)?;
        let patch = json_patch::diff(&left, &right);
        Ok(ServerSignalUpdate {
            name: name.into(),
            patch,
        })
    }

    /// Creates a new [`ServerSignalUpdate`] from two json values.
    pub fn new_from_json(name: impl Into<Cow<'static, str>>, old: &Value, new: &Value) -> Self {
        let patch = json_patch::diff(old, new);
        ServerSignalUpdate {
            name: name.into(),
            patch,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        id: u32,
        name: String,
        value: i32,
    }

    #[test]
    fn test_server_signal_update_new() {
        // Arrange
        let old = TestStruct {
            id: 1,
            name: "old".to_string(),
            value: 10,
        };
        let new = TestStruct {
            id: 1,
            name: "new".to_string(),
            value: 20,
        };

        // Act
        let update = ServerSignalUpdate::new("test_signal", &old, &new).unwrap();

        // Assert
        assert_eq!(update.name, "test_signal");
        assert!(!update.patch.0.is_empty());
    }

    #[test]
    fn test_server_signal_update_new_from_json() {
        // Arrange
        let old = json!({"id": 1, "name": "old", "value": 10});
        let new = json!({"id": 1, "name": "new", "value": 20});

        // Act
        let update = ServerSignalUpdate::new_from_json("test_signal", &old, &new);

        // Assert
        assert_eq!(update.name, "test_signal");
        assert!(!update.patch.0.is_empty());
    }

    #[test]
    fn test_server_signal_update_no_changes() {
        // Arrange
        let data = TestStruct {
            id: 1,
            name: "test".to_string(),
            value: 10,
        };

        // Act
        let update = ServerSignalUpdate::new("test_signal", &data, &data).unwrap();

        // Assert
        assert_eq!(update.name, "test_signal");
        assert!(update.patch.0.is_empty());
    }

    #[test]
    fn test_server_signal_update_serialization() {
        // Arrange
        let old = json!({"value": 10});
        let new = json!({"value": 20});
        let update = ServerSignalUpdate::new_from_json("test_signal", &old, &new);

        // Act
        let serialized = serde_json::to_string(&update).unwrap();
        let deserialized: ServerSignalUpdate = serde_json::from_str(&serialized).unwrap();

        // Assert
        assert_eq!(update.name, deserialized.name);
        assert_eq!(update.patch, deserialized.patch);
    }

    #[test]
    fn test_messages_serialization() {
        // Arrange
        let establish_msg = Messages::ServerSignal(ServerSignalMessage::Establish("test".to_string()));
        let old = json!({"value": 10});
        let new = json!({"value": 20});
        let update = ServerSignalUpdate::new_from_json("test_signal", &old, &new);
        let update_msg = Messages::ServerSignal(ServerSignalMessage::Update(update));

        // Act & Assert - Establish message
        let serialized = serde_json::to_string(&establish_msg).unwrap();
        let deserialized: Messages = serde_json::from_str(&serialized).unwrap();
        assert_eq!(establish_msg, deserialized);

        // Act & Assert - Update message
        let serialized = serde_json::to_string(&update_msg).unwrap();
        let deserialized: Messages = serde_json::from_str(&serialized).unwrap();
        assert_eq!(update_msg, deserialized);
    }

    #[test]
    fn test_server_signal_message_variants() {
        // Test Establish variant
        let establish = ServerSignalMessage::Establish("test_signal".to_string());
        let serialized = serde_json::to_string(&establish).unwrap();
        let deserialized: ServerSignalMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(establish, deserialized);

        // Test EstablishResponse variant
        let response = ServerSignalMessage::EstablishResponse((
            "test_signal".to_string(),
            json!({"value": 42}),
        ));
        let serialized = serde_json::to_string(&response).unwrap();
        let deserialized: ServerSignalMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response, deserialized);

        // Test Update variant
        let old = json!({"value": 10});
        let new = json!({"value": 20});
        let update = ServerSignalUpdate::new_from_json("test_signal", &old, &new);
        let update_msg = ServerSignalMessage::Update(update);
        let serialized = serde_json::to_string(&update_msg).unwrap();
        let deserialized: ServerSignalMessage = serde_json::from_str(&serialized).unwrap();
        assert_eq!(update_msg, deserialized);
    }
}
