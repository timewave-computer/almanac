use almanac_core::{Error, Result};

#[test]
fn test_error_creation() {
    // Test storage error
    let err = Error::storage("database connection failed");
    assert!(matches!(err, Error::Storage(_)));
    
    // Test chain error
    let err = Error::chain("ethereum", "connection failed");
    assert!(matches!(err, Error::Chain { .. }));
    
    // Test event processing error
    let err = Error::event_processing("invalid event");
    assert!(matches!(err, Error::EventProcessing(_)));
    
    // Test configuration error
    let err = Error::configuration("invalid config");
    assert!(matches!(err, Error::Configuration(_)));
    
    // Test generic error
    let err = Error::generic("something went wrong");
    assert!(matches!(err, Error::Generic(_)));
}

#[test]
fn test_error_display() {
    let err = Error::storage("test error");
    assert_eq!(format!("{}", err), "Storage error: test error");
    
    let err = Error::chain("ethereum", "connection failed");
    assert_eq!(format!("{}", err), "Chain error for ethereum: connection failed");
    
    let err = Error::event_processing("invalid event");
    assert_eq!(format!("{}", err), "Event processing error: invalid event");
    
    let err = Error::configuration("invalid config");
    assert_eq!(format!("{}", err), "Configuration error: invalid config");
    
    let err = Error::generic("something went wrong");
    assert_eq!(format!("{}", err), "something went wrong");
}

#[test]
fn test_result_type() {
    let success = 42;
    assert_eq!(success, 42);
    
    let failure: Result<i32> = Err(Error::storage("test"));
    assert!(failure.is_err());
}

#[test]
fn test_error_chain_variant() {
    let err = Error::chain("ethereum", "rpc timeout");
    if let Error::Chain { chain, message } = err {
        assert_eq!(chain, "ethereum");
        assert_eq!(message, "rpc timeout");
    } else {
        panic!("Expected Chain error");
    }
}

#[test]
fn test_error_conversion() {
    // Test serde_json::Error conversion
    let json_err = serde_json::from_str::<i32>("invalid json");
    assert!(json_err.is_err());
    
    let almanac_err: Error = json_err.unwrap_err().into();
    assert!(matches!(almanac_err, Error::Serialization(_)));
}

#[test]
fn test_error_helpers() {
    // Test all error creation helpers
    let storage_err = Error::storage("db error");
    assert!(matches!(storage_err, Error::Storage(_)));
    
    let config_err = Error::configuration("bad config");
    assert!(matches!(config_err, Error::Configuration(_)));
    
    let chain_err = Error::chain("cosmos", "timeout");
    assert!(matches!(chain_err, Error::Chain { .. }));
    
    let event_err = Error::event_processing("parse error");
    assert!(matches!(event_err, Error::EventProcessing(_)));
    
    let generic_err = Error::generic("generic error");
    assert!(matches!(generic_err, Error::Generic(_)));
}