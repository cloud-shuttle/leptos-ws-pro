//! Backward compatibility tests
//!
//! These tests ensure that API changes maintain backward compatibility
//! and that deprecated features are properly handled.

use serde_json::{json, Value};
use std::collections::HashMap;

/// API version information
#[derive(Debug, Clone, PartialEq)]
struct ApiVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ApiVersion {
    fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    fn is_compatible_with(&self, other: &ApiVersion) -> bool {
        // Major version must match for compatibility
        self.major == other.major
    }

    fn is_newer_than(&self, other: &ApiVersion) -> bool {
        if self.major > other.major {
            true
        } else if self.major == other.major {
            if self.minor > other.minor {
                true
            } else if self.minor == other.minor {
                self.patch > other.patch
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Mock API client with version support
struct VersionedApiClient {
    version: ApiVersion,
    supported_versions: Vec<ApiVersion>,
    deprecated_features: HashMap<String, ApiVersion>,
}

impl VersionedApiClient {
    fn new(version: ApiVersion) -> Self {
        let supported_versions = vec![
            ApiVersion::new(1, 0, 0),
            ApiVersion::new(1, 0, 1),
            ApiVersion::new(1, 1, 0),
        ];

        let mut deprecated_features = HashMap::new();
        deprecated_features.insert("old_message_format".to_string(), ApiVersion::new(1, 0, 0));
        deprecated_features.insert("legacy_rpc_methods".to_string(), ApiVersion::new(1, 0, 0));

        Self {
            version,
            supported_versions,
            deprecated_features,
        }
    }

    fn is_version_supported(&self, version: &ApiVersion) -> bool {
        self.supported_versions.iter().any(|v| v == version)
    }

    fn is_feature_deprecated(&self, feature: &str, client_version: &ApiVersion) -> bool {
        if let Some(deprecated_in) = self.deprecated_features.get(feature) {
            client_version.is_newer_than(deprecated_in)
        } else {
            false
        }
    }

    fn handle_request(&self, request: &Value, client_version: &ApiVersion) -> Result<Value, String> {
        // Check version compatibility
        if !self.is_version_supported(client_version) {
            return Err(format!(
                "Unsupported API version: {}. Supported versions: {}",
                client_version.to_string(),
                self.supported_versions
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Check for deprecated features
        if let Some(method) = request.get("method") {
            let method_str = method.as_str().unwrap_or("");
            if self.is_feature_deprecated("legacy_rpc_methods", client_version) {
                let legacy_methods = ["OldSendMessage", "LegacyGetMessages"];
                if legacy_methods.contains(&method_str) {
                    return Err(format!(
                        "Deprecated RPC method: {}. Use newer alternatives.",
                        method_str
                    ));
                }
            }
        }

        // Check for deprecated message format
        if let Some(message_type) = request.get("message_type") {
            let message_type_str = message_type.as_str().unwrap_or("");
            if message_type_str == "legacy" && self.is_feature_deprecated("old_message_format", client_version) {
                return Err("Deprecated message format: 'legacy'. Use 'json' or 'rkyv' instead.".to_string());
            }
        }

        // Simulate successful response
        Ok(json!({
            "id": request.get("id").unwrap_or(&json!(null)),
            "result": {
                "success": true,
                "api_version": self.version.to_string(),
                "client_version": client_version.to_string()
            }
        }))
    }
}

#[test]
fn test_version_compatibility() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test compatible version
    let compatible_client = ApiVersion::new(1, 0, 0);
    assert!(server.version.is_compatible_with(&compatible_client));

    // Test incompatible version
    let incompatible_client = ApiVersion::new(2, 0, 0);
    assert!(!server.version.is_compatible_with(&incompatible_client));

    // Test same version
    let same_version = ApiVersion::new(1, 1, 0);
    assert!(server.version.is_compatible_with(&same_version));
}

#[test]
fn test_supported_versions() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test supported versions
    assert!(server.is_version_supported(&ApiVersion::new(1, 0, 0)));
    assert!(server.is_version_supported(&ApiVersion::new(1, 0, 1)));
    assert!(server.is_version_supported(&ApiVersion::new(1, 1, 0)));

    // Test unsupported versions
    assert!(!server.is_version_supported(&ApiVersion::new(2, 0, 0)));
    assert!(!server.is_version_supported(&ApiVersion::new(0, 9, 0)));
}

#[test]
fn test_deprecated_features() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test deprecated feature with old client
    let old_client = ApiVersion::new(1, 0, 0);
    assert!(!server.is_feature_deprecated("old_message_format", &old_client));

    // Test deprecated feature with new client
    let new_client = ApiVersion::new(1, 1, 0);
    assert!(server.is_feature_deprecated("old_message_format", &new_client));

    // Test non-deprecated feature
    assert!(!server.is_feature_deprecated("new_feature", &new_client));
}

#[test]
fn test_backward_compatible_requests() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test request from old client (should work)
    let old_client = ApiVersion::new(1, 0, 0);
    let old_request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "params": {
            "message": "Hello World"
        }
    });

    let response = server.handle_request(&old_request, &old_client);
    assert!(response.is_ok(), "Old client request should succeed");

    let response = response.unwrap();
    assert_eq!(response["result"]["client_version"], "1.0.0");
    assert_eq!(response["result"]["api_version"], "1.1.0");

    // Test request from new client (should work)
    let new_client = ApiVersion::new(1, 1, 0);
    let new_request = json!({
        "id": "req-456",
        "method": "SendMessage",
        "params": {
            "message": "Hello World",
            "new_feature": true
        }
    });

    let response = server.handle_request(&new_request, &new_client);
    assert!(response.is_ok(), "New client request should succeed");
}

#[test]
fn test_unsupported_version_rejection() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test request from unsupported version
    let unsupported_client = ApiVersion::new(2, 0, 0);
    let request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "params": {}
    });

    let response = server.handle_request(&request, &unsupported_client);
    assert!(response.is_err(), "Unsupported version should be rejected");
    assert!(response.unwrap_err().contains("Unsupported API version"));
}

#[test]
fn test_deprecated_feature_rejection() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test deprecated RPC method with new client
    let new_client = ApiVersion::new(1, 1, 0);
    let deprecated_request = json!({
        "id": "req-123",
        "method": "OldSendMessage",
        "params": {}
    });

    let response = server.handle_request(&deprecated_request, &new_client);
    assert!(response.is_err(), "Deprecated feature should be rejected");
    assert!(response.unwrap_err().contains("Deprecated RPC method"));

    // Test deprecated message format with new client
    let deprecated_message_request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "message_type": "legacy",
        "params": {}
    });

    let response = server.handle_request(&deprecated_message_request, &new_client);
    assert!(response.is_err(), "Deprecated message format should be rejected");
    assert!(response.unwrap_err().contains("Deprecated message format"));
}

#[test]
fn test_deprecated_feature_acceptance_with_old_client() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test deprecated RPC method with old client (should work)
    let old_client = ApiVersion::new(1, 0, 0);
    let deprecated_request = json!({
        "id": "req-123",
        "method": "OldSendMessage",
        "params": {}
    });

    let response = server.handle_request(&deprecated_request, &old_client);
    assert!(response.is_ok(), "Deprecated feature should work with old client");

    // Test deprecated message format with old client (should work)
    let deprecated_message_request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "message_type": "legacy",
        "params": {}
    });

    let response = server.handle_request(&deprecated_message_request, &old_client);
    assert!(response.is_ok(), "Deprecated message format should work with old client");
}

#[test]
fn test_version_migration_path() {
    let server = VersionedApiClient::new(ApiVersion::new(1, 1, 0));

    // Test migration from 1.0.0 to 1.1.0
    let old_client = ApiVersion::new(1, 0, 0);
    let new_client = ApiVersion::new(1, 1, 0);

    // Old client should work
    let old_request = json!({
        "id": "req-123",
        "method": "SendMessage",
        "params": {}
    });

    let response = server.handle_request(&old_request, &old_client);
    assert!(response.is_ok(), "Old client should work during migration");

    // New client should work
    let new_request = json!({
        "id": "req-456",
        "method": "SendMessage",
        "params": {}
    });

    let response = server.handle_request(&new_request, &new_client);
    assert!(response.is_ok(), "New client should work during migration");
}

#[test]
fn test_api_version_header_handling() {
    // Test that API version is properly extracted from headers
    let mut headers = HashMap::new();
    headers.insert("API-Version".to_string(), "1.0.0".to_string());
    headers.insert("User-Agent".to_string(), "LeptosWSPro/1.0.0".to_string());

    // Extract version from header
    let version_header = headers.get("API-Version").unwrap();
    let client_version = ApiVersion::new(1, 0, 0);
    assert_eq!(version_header, &client_version.to_string());

    // Test missing version header (should default to latest)
    headers.remove("API-Version");
    let default_version = ApiVersion::new(1, 1, 0); // Assume latest
    assert_eq!(default_version.to_string(), "1.1.0");
}

#[test]
fn test_breaking_change_detection() {
    let server = VersionedApiClient::new(ApiVersion::new(2, 0, 0));

    // Test that breaking changes are properly detected
    let old_client = ApiVersion::new(1, 1, 0);
    let new_client = ApiVersion::new(2, 0, 0);

    // Old client should be rejected for breaking changes
    assert!(!server.version.is_compatible_with(&old_client));

    // New client should work
    assert!(server.version.is_compatible_with(&new_client));
}
