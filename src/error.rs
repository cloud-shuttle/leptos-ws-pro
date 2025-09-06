use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("No ServerSignals in State")]
    MissingServerSignals,
    #[error("Could not add ServerSignal to ServerSignals")]
    AddingSignalFailed,
    #[error("Could not update Signal")]
    UpdateSignalFailed,

    #[error(transparent)]
    SerializationFailed(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_error_display_messages() {
        // Test MissingServerSignals
        let error = Error::MissingServerSignals;
        assert_eq!(error.to_string(), "No ServerSignals in State");

        // Test AddingSignalFailed
        let error = Error::AddingSignalFailed;
        assert_eq!(
            error.to_string(),
            "Could not add ServerSignal to ServerSignals"
        );

        // Test UpdateSignalFailed
        let error = Error::UpdateSignalFailed;
        assert_eq!(error.to_string(), "Could not update Signal");
    }

    #[test]
    fn test_serialization_error_conversion() {
        // Arrange
        let json_error =
            serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "Test error"));

        // Act
        let error = Error::from(json_error);

        // Assert
        match error {
            Error::SerializationFailed(serde_error) => {
                assert!(serde_error.to_string().contains("Test error"));
            }
            _ => panic!("Expected SerializationFailed variant"),
        }
    }

    #[test]
    fn test_error_debug_formatting() {
        let error = Error::MissingServerSignals;
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("MissingServerSignals"));
    }

    #[test]
    fn test_error_equality() {
        let error1 = Error::MissingServerSignals;
        let error2 = Error::MissingServerSignals;
        let error3 = Error::AddingSignalFailed;

        // Note: Error doesn't implement PartialEq, so we test the debug representation
        assert_eq!(format!("{:?}", error1), format!("{:?}", error2));
        assert_ne!(format!("{:?}", error1), format!("{:?}", error3));
    }

    #[test]
    fn test_error_chain() {
        // Test that we can chain errors properly
        let json_error =
            serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "Chain test"));
        let error: Error = json_error.into();

        // Verify the error chain works
        let error_string = error.to_string();
        assert!(error_string.contains("Chain test"));
    }
}
