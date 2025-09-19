//! Server Signal Implementation
//!
//! Main signal implementation for server-side reactive state

use std::any::Any;
use std::ops::{Deref, DerefMut};
use std::panic::Location;
use std::sync::Arc;

use crate::error::Error;
use crate::messages::ServerSignalUpdate;
use crate::server_signals::ServerSignals;
use futures::executor::block_on;
use guards::{Plain, ReadGuard};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::sync::broadcast::{Receiver, Sender, channel};

use super::traits::ServerSignalTrait;

/// A signal owned by the server which writes to the websocket when mutated.
#[derive(Clone, Debug)]
pub struct ServerSignal<T>
where
    T: Clone + Send + Sync + for<'de> Deserialize<'de>,
{
    initial: T,
    name: String,
    value: ArcRwSignal<T>,
    json_value: Arc<RwLock<Value>>,
    observers: Arc<Sender<ServerSignalUpdate>>,
}

impl<T> ServerSignal<T>
where
    T: Clone + Serialize + Send + Sync + for<'de> Deserialize<'de> + 'static,
{
    pub fn new(name: String, value: T) -> Result<Self, Error> {
        let mut signals = use_context::<ServerSignals>().ok_or(Error::MissingServerSignals)?;
        if block_on(signals.contains(&name)) {
            return Ok(block_on(signals.get_signal::<ServerSignal<T>>(name)).unwrap());
        }
        let (send, _) = channel(32);
        let new_signal = ServerSignal {
            initial: value.clone(),
            name: name.clone(),
            value: create_rw_signal(value),
            json_value: Arc::new(RwLock::new(serde_json::to_value(&value)?)),
            observers: Arc::new(send),
        };
        block_on(signals.register_signal(name, new_signal.clone()))?;
        Ok(new_signal)
    }

    pub fn subscribe(&self) -> Receiver<ServerSignalUpdate> {
        self.observers.subscribe()
    }

    pub fn get(&self) -> ReadGuard<T, Plain> {
        self.value.get()
    }

    pub fn set(&self, value: T) -> Result<(), Error> {
        self.value.set(value.clone());
        let new_json = serde_json::to_value(&value)?;
        let current_json = block_on(self.json_value.read()).clone();

        if current_json != new_json {
            let update = ServerSignalUpdate::new_from_json(
                self.name.clone(),
                &current_json,
                &new_json,
            );
            let _ = self.observers.send(update);
            *block_on(self.json_value.write()) = new_json;
        }
        Ok(())
    }

    pub fn update<F>(&self, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut T),
    {
        let mut current = self.value.get().clone();
        f(&mut current);
        self.set(current)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initial(&self) -> T {
        self.initial.clone()
    }
}

impl<T> Update for ServerSignal<T>
where
    T: Clone + Send + Sync + for<'de> Deserialize<'de>,
{
    fn update(&self) {
        self.value.update();
    }
}

impl<T> DefinedAt for ServerSignal<T>
where
    T: Clone + Send + Sync + for<'de> Deserialize<'de>,
{
    fn defined_at(&self) -> Location<'static> {
        Location::caller()
    }
}

impl<T> ReadUntracked for ServerSignal<T>
where
    T: Clone + Send + Sync + for<'de> Deserialize<'de>,
{
    type Value = T;

    fn read_untracked(&self) -> Self::Value {
        self.value.read_untracked()
    }
}

impl<T> Get for ServerSignal<T>
where
    T: Clone + Send + Sync + for<'de> Deserialize<'de>,
{
    type Value = T;

    fn get(&self) -> ReadGuard<Self::Value, Plain> {
        self.value.get()
    }
}

impl<T> Deref for ServerSignal<T>
where
    T: Clone + Send + Sync + for<'de> Deserialize<'de>,
{
    type Target = ArcRwSignal<T>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
