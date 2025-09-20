# Server-Sent Events (SSE) Client Design

## Overview

Focused design for SSE client implementation with automatic reconnection and event parsing.

## Architecture

### Core Client Structure

```rust
pub struct SseClient {
    config: SseConfig,
    state: Arc<Mutex<ConnectionState>>,
    event_stream: Option<EventStream>,
    reconnect_handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct SseConfig {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub last_event_id: Option<String>,
}
```

### Event Stream Implementation

```rust
use eventsource_stream::{Eventsource, EventStreamError};
use futures_util::StreamExt;

pub struct EventStream {
    stream: Pin<Box<dyn Stream<Item = Result<SseEvent, EventStreamError>> + Send>>,
    last_event_id: Option<String>,
}

impl EventStream {
    pub async fn connect(config: &SseConfig) -> Result<Self, SseError> {
        let client = reqwest::Client::builder()
            .timeout(config.connect_timeout)
            .build()?;

        let mut request = client.get(&config.url);

        // Add headers
        for (key, value) in &config.headers {
            request = request.header(key, value);
        }

        // Add Last-Event-ID if present
        if let Some(last_id) = &config.last_event_id {
            request = request.header("Last-Event-ID", last_id);
        }

        let response = request.send().await?;
        let stream = response.bytes_stream().eventsource();

        Ok(Self {
            stream: Box::pin(stream),
            last_event_id: config.last_event_id.clone(),
        })
    }
}
```

### Event Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct SseEvent {
    pub event_type: String,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

impl SseEvent {
    pub fn parse_data<T>(&self) -> Result<T, serde_json::Error>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str(&self.data)
    }
}

#[derive(Debug, Clone)]
pub enum SseEventType {
    Message,
    Open,
    Error,
    Custom(String),
}
```

## Connection Management

### Connection States

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected {
        connected_at: Instant,
        events_received: u64,
    },
    Reconnecting {
        attempt: u32,
        next_retry: Instant,
        last_error: Option<Arc<SseError>>,
    },
    Failed {
        error: Arc<SseError>,
        final_attempt: bool,
    },
}
```

### Automatic Reconnection

```rust
impl SseClient {
    pub async fn connect(&mut self) -> Result<(), SseError> {
        *self.state.lock().await = ConnectionState::Connecting;

        match EventStream::connect(&self.config).await {
            Ok(stream) => {
                *self.state.lock().await = ConnectionState::Connected {
                    connected_at: Instant::now(),
                    events_received: 0,
                };

                self.event_stream = Some(stream);
                self.start_event_loop();
                Ok(())
            }
            Err(err) => {
                self.schedule_reconnect(1, err.clone()).await;
                Err(err)
            }
        }
    }

    async fn schedule_reconnect(&mut self, attempt: u32, error: SseError) {
        if attempt > self.config.max_reconnect_attempts {
            *self.state.lock().await = ConnectionState::Failed {
                error: Arc::new(error),
                final_attempt: true,
            };
            return;
        }

        let delay = self.calculate_backoff_delay(attempt);
        *self.state.lock().await = ConnectionState::Reconnecting {
            attempt,
            next_retry: Instant::now() + delay,
            last_error: Some(Arc::new(error)),
        };

        // Schedule reconnection attempt
        let state_clone = Arc::clone(&self.state);
        let config_clone = self.config.clone();

        self.reconnect_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            // Trigger reconnection...
        }));
    }

    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.reconnect_interval;
        let exponential_delay = base_delay.mul_f64(2.0_f64.powi(attempt as i32 - 1));

        // Cap at maximum delay and add jitter
        let max_delay = Duration::from_secs(300); // 5 minutes max
        let delay = std::cmp::min(exponential_delay, max_delay);

        // Add random jitter (±25%)
        let jitter = delay.mul_f64(0.25) * fastrand::f64();
        delay + Duration::from_nanos(jitter as u64)
    }
}
```

## Event Processing

### Event Loop

```rust
impl SseClient {
    fn start_event_loop(&mut self) {
        if let Some(ref mut stream) = self.event_stream {
            let state = Arc::clone(&self.state);
            let config = self.config.clone();

            tokio::spawn(async move {
                while let Some(event_result) = stream.stream.next().await {
                    match event_result {
                        Ok(event) => {
                            // Update last event ID for reconnection
                            if let Some(ref id) = event.id {
                                stream.last_event_id = Some(id.clone());
                            }

                            // Handle retry instruction
                            if let Some(retry_ms) = event.retry {
                                config.reconnect_interval = Duration::from_millis(retry_ms);
                            }

                            // Update connection state
                            if let ConnectionState::Connected { events_received, .. } =
                                &mut *state.lock().await
                            {
                                *events_received += 1;
                            }

                            // Process event
                            Self::handle_event(event).await;
                        }
                        Err(err) => {
                            // Connection error - trigger reconnection
                            Self::handle_connection_error(err, &state, &config).await;
                            break;
                        }
                    }
                }
            });
        }
    }

    async fn handle_event(event: SseEvent) {
        match event.event_type.as_str() {
            "message" | "" => {
                // Default message event
                println!("Received message: {}", event.data);
            }
            "heartbeat" => {
                // Server heartbeat - no action needed
            }
            "error" => {
                // Server-sent error
                eprintln!("Server error: {}", event.data);
            }
            custom_type => {
                // Custom event type
                println!("Custom event {}: {}", custom_type, event.data);
            }
        }
    }
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SseError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("HTTP error: {status}")]
    Http { status: u16, body: String },

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Timeout: {operation} after {duration:?}")]
    Timeout { operation: String, duration: Duration },

    #[error("Max reconnect attempts ({max}) exceeded")]
    ReconnectFailed { max: u32 },

    #[error("Stream error: {0}")]
    Stream(#[from] EventStreamError),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Invalid event format")]
    InvalidEvent,
}
```

### Event Filtering and Callbacks

```rust
pub struct EventFilter {
    pub event_types: HashSet<String>,
    pub callback: Box<dyn Fn(SseEvent) -> Result<(), Box<dyn std::error::Error>> + Send + Sync>,
}

impl SseClient {
    pub fn add_event_listener<F>(&mut self, event_type: String, callback: F)
    where
        F: Fn(SseEvent) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static,
    {
        let filter = EventFilter {
            event_types: [event_type].into_iter().collect(),
            callback: Box::new(callback),
        };
        self.event_filters.push(filter);
    }

    pub fn remove_event_listener(&mut self, event_type: &str) {
        self.event_filters.retain(|filter| !filter.event_types.contains(event_type));
    }
}
```

## Testing Strategy

### Unit Tests

- Event parsing from raw SSE format
- Connection state transitions
- Reconnection backoff calculation
- Error handling scenarios
- Event filtering logic

### Integration Tests

- Real SSE server connection
- Network failure simulation
- Long-running connection stability
- Large event handling
- Concurrent event processing

## Key Design Decisions

1. **Read-Only**: SSE is unidirectional (server-to-client only)
2. **Automatic Reconnection**: Built-in exponential backoff
3. **Event ID Tracking**: Maintains last event ID for resumption
4. **Streaming**: Uses async streams for efficient processing
5. **Error Recovery**: Graceful handling of connection failures

## Implementation Files (<300 lines each)

- `sse/client.rs` - Core SSE client
- `sse/events.rs` - Event parsing and types
- `sse/connection.rs` - Connection management
- `sse/errors.rs` - Error definitions
- `sse/config.rs` - Configuration types
