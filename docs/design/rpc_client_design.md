# RPC Client Design

## Overview

Type-safe RPC client with automatic request correlation and transport abstraction.

## Architecture

### Core RPC Client

```rust
pub struct RpcClient {
    transport: Box<dyn Transport>,
    correlations: CorrelationManager,
    config: RpcConfig,
    metrics: Arc<RpcMetrics>,
}

#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub default_timeout: Duration,
    pub max_concurrent_requests: usize,
    pub retry_config: RetryConfig,
    pub compression: bool,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            max_concurrent_requests: 1000,
            retry_config: RetryConfig::default(),
            compression: false,
        }
    }
}
```

### Request/Response Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest<T> {
    pub id: RequestId,
    pub method: String,
    pub params: T,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub id: RequestId,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

pub type RequestId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
```

### Correlation Management

```rust
pub struct CorrelationManager {
    pending_requests: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,
    id_generator: Arc<Mutex<RequestIdGenerator>>,
}

struct PendingRequest {
    sender: oneshot::Sender<Result<serde_json::Value, RpcError>>,
    timeout: Option<Instant>,
    method: String,
    created_at: Instant,
}

impl CorrelationManager {
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            id_generator: Arc::new(Mutex::new(RequestIdGenerator::new())),
        }
    }

    pub fn generate_request_id(&self) -> RequestId {
        self.id_generator.lock().unwrap().next_id()
    }

    pub async fn register_request(
        &self,
        request_id: RequestId,
        method: String,
        timeout: Option<Duration>,
    ) -> oneshot::Receiver<Result<serde_json::Value, RpcError>> {
        let (sender, receiver) = oneshot::channel();
        let timeout_instant = timeout.map(|t| Instant::now() + t);

        let request = PendingRequest {
            sender,
            timeout: timeout_instant,
            method,
            created_at: Instant::now(),
        };

        self.pending_requests
            .lock()
            .await
            .insert(request_id, request);

        receiver
    }

    pub async fn complete_request(
        &self,
        request_id: &RequestId,
        result: Result<serde_json::Value, RpcError>,
    ) -> bool {
        if let Some(pending) = self.pending_requests
            .lock()
            .await
            .remove(request_id)
        {
            let _ = pending.sender.send(result);
            true
        } else {
            false
        }
    }
}
```

## RPC Method Implementation

### Core RPC Methods

```rust
impl RpcClient {
    pub async fn call<Params, Result>(
        &mut self,
        method: &str,
        params: Params,
    ) -> Result<Result, RpcClientError>
    where
        Params: Serialize + Send,
        Result: DeserializeOwned + Send,
    {
        self.call_with_timeout(method, params, self.config.default_timeout).await
    }

    pub async fn call_with_timeout<Params, Result>(
        &mut self,
        method: &str,
        params: Params,
        timeout: Duration,
    ) -> Result<Result, RpcClientError>
    where
        Params: Serialize + Send,
        Result: DeserializeOwned + Send,
    {
        let request_id = self.correlations.generate_request_id();

        // Register for response correlation
        let response_receiver = self.correlations
            .register_request(request_id.clone(), method.to_string(), Some(timeout))
            .await;

        // Build and send request
        let request = RpcRequest {
            id: request_id.clone(),
            method: method.to_string(),
            params,
            timeout: Some(timeout),
        };

        let serialized = serde_json::to_vec(&request)
            .map_err(|e| RpcClientError::Serialization(e.to_string()))?;

        // Send via transport
        self.transport
            .send(Message::Binary(serialized))
            .await
            .map_err(RpcClientError::Transport)?;

        // Wait for response with timeout
        let response_result = tokio::time::timeout(timeout, response_receiver)
            .await
            .map_err(|_| RpcClientError::Timeout {
                method: method.to_string(),
                timeout,
            })?
            .map_err(|_| RpcClientError::CorrelationCancelled)?;

        match response_result {
            Ok(response_value) => {
                serde_json::from_value(response_value)
                    .map_err(|e| RpcClientError::Deserialization(e.to_string()))
            }
            Err(rpc_error) => Err(RpcClientError::Rpc(rpc_error)),
        }
    }

    pub async fn notify<Params>(&mut self, method: &str, params: Params) -> Result<(), RpcClientError>
    where
        Params: Serialize + Send,
    {
        let request = RpcRequest {
            id: "".to_string(), // Empty ID for notifications
            method: method.to_string(),
            params,
            timeout: None,
        };

        let serialized = serde_json::to_vec(&request)
            .map_err(|e| RpcClientError::Serialization(e.to_string()))?;

        self.transport
            .send(Message::Binary(serialized))
            .await
            .map_err(RpcClientError::Transport)?;

        Ok(())
    }
}
```

### Response Processing

```rust
impl RpcClient {
    pub async fn handle_message(&mut self, message: Message) -> Result<(), RpcClientError> {
        let data = match message {
            Message::Binary(data) => data,
            Message::Text(text) => text.into_bytes(),
            _ => return Ok(()), // Ignore non-data messages
        };

        // Parse response
        let response: RpcResponse<serde_json::Value> = serde_json::from_slice(&data)
            .map_err(|e| RpcClientError::Deserialization(e.to_string()))?;

        // Route to waiting request
        let result = match (response.result, response.error) {
            (Some(result), None) => Ok(result),
            (None, Some(error)) => Err(error),
            (Some(_), Some(_)) => Err(RpcError {
                code: -32603,
                message: "Invalid response: both result and error present".to_string(),
                data: None,
            }),
            (None, None) => Err(RpcError {
                code: -32603,
                message: "Invalid response: neither result nor error present".to_string(),
                data: None,
            }),
        };

        if !self.correlations.complete_request(&response.id, result).await {
            // Orphaned response - log warning
            tracing::warn!("Received response for unknown request ID: {}", response.id);
        }

        Ok(())
    }

    async fn start_message_processor(&mut self) {
        // Spawn task to process incoming transport messages
        let mut transport_receiver = self.transport.subscribe().await;

        while let Some(message) = transport_receiver.recv().await {
            if let Err(err) = self.handle_message(message).await {
                tracing::error!("Failed to process RPC message: {:?}", err);
            }
        }
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("Transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("RPC error: {0:?}")]
    Rpc(RpcError),

    #[error("Timeout after {timeout:?} calling method '{method}'")]
    Timeout { method: String, timeout: Duration },

    #[error("Request correlation was cancelled")]
    CorrelationCancelled,

    #[error("Too many concurrent requests (max: {max})")]
    TooManyRequests { max: usize },

    #[error("Client not connected")]
    NotConnected,
}
```

### Retry Logic

```rust
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub retry_on: Vec<RpcErrorCode>,
}

impl RpcClient {
    pub async fn call_with_retry<Params, Result>(
        &mut self,
        method: &str,
        params: Params,
        retry_config: RetryConfig,
    ) -> Result<Result, RpcClientError>
    where
        Params: Serialize + Send + Clone,
        Result: DeserializeOwned + Send,
    {
        let mut attempt = 0;
        let mut delay = retry_config.initial_delay;

        loop {
            attempt += 1;

            match self.call(method, params.clone()).await {
                Ok(result) => return Ok(result),
                Err(RpcClientError::Rpc(rpc_error)) if attempt < retry_config.max_attempts => {
                    if retry_config.retry_on.contains(&rpc_error.code) {
                        tokio::time::sleep(delay).await;
                        delay = std::cmp::min(
                            delay.mul_f64(retry_config.multiplier),
                            retry_config.max_delay,
                        );
                        continue;
                    } else {
                        return Err(RpcClientError::Rpc(rpc_error));
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests

- Request/response correlation
- Timeout handling
- Error scenarios
- Retry logic
- Serialization/deserialization

### Integration Tests

- Real transport integration
- Concurrent request handling
- Network failure scenarios
- Performance under load

## Key Design Decisions

1. **Transport Agnostic**: Works with any transport implementation
2. **Type Safety**: Generic request/response types with serde
3. **Async First**: All operations are async
4. **Correlation**: Automatic request-response matching
5. **Error Recovery**: Built-in retry and timeout handling

## Implementation Files (<300 lines each)

- `rpc/client.rs` - Core RPC client
- `rpc/correlation.rs` - Request correlation management
- `rpc/types.rs` - Request/response type definitions
- `rpc/errors.rs` - Error types and handling
- `rpc/retry.rs` - Retry logic and configuration
