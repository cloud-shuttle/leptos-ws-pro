use leptos_ws::transport::{TransportConfig, TransportFactory};

#[tokio::test]
async fn test_transport_config_creation() {
    let config = TransportConfig {
        url: "ws://localhost:8080".to_string(),
        ..Default::default()
    };
    
    assert_eq!(config.url, "ws://localhost:8080");
    assert_eq!(config.timeout.as_secs(), 30);
}

#[test]
fn test_transport_capabilities_detection() {
    let caps = leptos_ws::transport::TransportCapabilities::detect();
    
    #[cfg(target_arch = "wasm32")]
    {
        assert!(caps.websocket);
        assert!(caps.sse);
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert!(caps.websocket);
        assert!(caps.sse);
        assert!(caps.compression);
    }
}
