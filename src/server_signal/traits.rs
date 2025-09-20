//! Server Signal Traits
//!
//! Trait definitions for server signal functionality

use async_trait::async_trait;
use serde_json::Value;
use std::any::Any;

use crate::error::Error;
use crate::messages::ServerSignalUpdate;

/// Trait for server signal functionality
#[async_trait]
pub trait ServerSignalTrait {
    async fn add_observer(&self) -> tokio::sync::broadcast::Receiver<ServerSignalUpdate>;
    async fn update_json(&self, patch: ServerSignalUpdate) -> Result<(), Error>;
    async fn update_if_changed(&self) -> Result<(), Error>;
    fn json(&self) -> Result<Value, Error>;
    fn as_any(&self) -> &dyn Any;
    fn track(&self);
}
