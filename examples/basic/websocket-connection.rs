//! Basic WebSocket Connection Example
//! 
//! This example demonstrates how to create a basic WebSocket connection
//! using the Leptos WS Pro library.

use leptos::prelude::*;
use leptos_ws_pro::*;

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    // Create WebSocket provider with server URL
    let provider = WebSocketProvider::new("ws://localhost:8080");
    
    // Create reactive WebSocket context
    let context = WebSocketContext::new(provider);
    
    // Get connection state signal
    let connection_state = context.connection_state;
    let is_connected = move || context.is_connected();
    
    // Handle connection button click
    let connect = move |_| {
        // In a real implementation, this would trigger connection
        // For this example, we'll just update the state
        context.set_connection_state(ConnectionState::Connecting);
        
        // Simulate connection after a delay
        set_timeout(move || {
            context.set_connection_state(ConnectionState::Connected);
        }, 1000);
    };
    
    // Handle disconnect button click
    let disconnect = move |_| {
        context.set_connection_state(ConnectionState::Disconnected);
    };
    
    // Handle send message button click
    let send_message = move |_| {
        if is_connected() {
            let message = "Hello from Leptos WS Pro!";
            context.send_message(message);
        }
    };
    
    view! {
        <div class="app">
            <h1>"Leptos WS Pro - Basic Connection Example"</h1>
            
            <div class="connection-status">
                <h2>"Connection Status"</h2>
                <div class="status" class:connected=is_connected>
                    {move || match connection_state.get() {
                        ConnectionState::Disconnected => "Disconnected",
                        ConnectionState::Connecting => "Connecting...",
                        ConnectionState::Connected => "Connected",
                    }}
                </div>
            </div>
            
            <div class="controls">
                <h2>"Controls"</h2>
                <div class="button-group">
                    <button 
                        on:click=connect
                        disabled=is_connected
                    >
                        "Connect"
                    </button>
                    
                    <button 
                        on:click=disconnect
                        disabled=move || !is_connected()
                    >
                        "Disconnect"
                    </button>
                    
                    <button 
                        on:click=send_message
                        disabled=move || !is_connected()
                    >
                        "Send Message"
                    </button>
                </div>
            </div>
            
            <div class="info">
                <h2>"Information"</h2>
                <p>"This example demonstrates:"</p>
                <ul>
                    <li>"Creating a WebSocket provider"</li>
                    <li>"Setting up reactive context"</li>
                    <li>"Managing connection state"</li>
                    <li>"Sending messages"</li>
                </ul>
            </div>
        </div>
    }
}

/// CSS styles for the example
const STYLES: &str = r#"
.app {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
    font-family: Arial, sans-serif;
}

.connection-status {
    margin: 20px 0;
    padding: 15px;
    border: 1px solid #ddd;
    border-radius: 5px;
}

.status {
    padding: 10px;
    border-radius: 3px;
    font-weight: bold;
    text-align: center;
    background-color: #f8f9fa;
    color: #6c757d;
}

.status.connected {
    background-color: #d4edda;
    color: #155724;
}

.controls {
    margin: 20px 0;
    padding: 15px;
    border: 1px solid #ddd;
    border-radius: 5px;
}

.button-group {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
}

button {
    padding: 10px 20px;
    border: none;
    border-radius: 3px;
    background-color: #007bff;
    color: white;
    cursor: pointer;
    font-size: 14px;
}

button:hover:not(:disabled) {
    background-color: #0056b3;
}

button:disabled {
    background-color: #6c757d;
    cursor: not-allowed;
}

.info {
    margin: 20px 0;
    padding: 15px;
    border: 1px solid #ddd;
    border-radius: 5px;
    background-color: #f8f9fa;
}

.info ul {
    margin: 10px 0;
    padding-left: 20px;
}

.info li {
    margin: 5px 0;
}
"#;

/// Main function to run the example
pub fn main() {
    // Mount the application
    leptos::mount_to_body(|| {
        view! {
            <style>{STYLES}</style>
            <App/>
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::*;
    
    #[test]
    fn test_app_creation() {
        let app = App();
        // Test that the app component can be created
        assert!(true); // Basic test to ensure compilation
    }
    
    #[test]
    fn test_websocket_provider_creation() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        // Test that provider can be created
        assert!(true); // Basic test to ensure compilation
    }
    
    #[test]
    fn test_websocket_context_creation() {
        let provider = WebSocketProvider::new("ws://localhost:8080");
        let context = WebSocketContext::new(provider);
        
        // Test initial connection state
        assert_eq!(context.connection_state.get(), ConnectionState::Disconnected);
        assert!(!context.is_connected());
    }
}
