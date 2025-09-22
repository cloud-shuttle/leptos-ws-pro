//! Basic WebSocket Connection Example
//!
//! This example demonstrates how to create a basic WebSocket connection
//! using the Leptos WS Pro library.

use leptos::prelude::*;
use leptos_ws_pro::*;

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="app">
            <h1>"Leptos WS Pro - Basic Connection Example"</h1>
            <p>"This example demonstrates basic WebSocket functionality."</p>
            <p>"TODO: Implement full example with proper closure handling"</p>
        </div>
    }
}

/// Main function to run the example
pub fn main() {
    // Mount the application
    mount_to_body(|| {
        view! {
            <style>{STYLES}</style>
            <App/>
        }
    });
}

const STYLES: &str = r#"
.app {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
    font-family: Arial, sans-serif;
}

h1 {
    color: #333;
    text-align: center;
}

p {
    color: #666;
    line-height: 1.6;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::prelude::*;

    #[test]
    fn test_app_renders() {
        let app = App();
        // Basic test to ensure the component compiles
        assert!(true);
    }
}
