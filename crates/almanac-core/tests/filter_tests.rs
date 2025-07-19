use almanac_core::EventFilter;

#[test]
fn test_event_filter_new() {
    let filter = EventFilter::new();
    
    assert_eq!(filter.chain, None);
    assert_eq!(filter.block_range, None);
    assert_eq!(filter.time_range, None);
    assert_eq!(filter.event_types, None);
    assert_eq!(filter.addresses, None);
    assert_eq!(filter.tx_hashes, None);
    assert_eq!(filter.limit, None);
    assert_eq!(filter.offset, None);
}

#[test]
fn test_event_filter_default() {
    let filter = EventFilter::default();
    
    assert_eq!(filter.chain, None);
    assert_eq!(filter.block_range, None);
    assert_eq!(filter.time_range, None);
    assert_eq!(filter.event_types, None);
    assert_eq!(filter.addresses, None);
    assert_eq!(filter.tx_hashes, None);
    assert_eq!(filter.limit, None);
    assert_eq!(filter.offset, None);
}

#[test]
fn test_event_filter_with_chain() {
    let filter = EventFilter::new()
        .with_chain("ethereum");
    
    assert_eq!(filter.chain, Some("ethereum".to_string()));
}

#[test]
fn test_event_filter_with_block_range() {
    let filter = EventFilter::new()
        .with_block_range(100, 200);
    
    assert_eq!(filter.block_range, Some((100, 200)));
}

#[test]
fn test_event_filter_with_time_range() {
    let filter = EventFilter::new()
        .with_time_range(1000, 2000);
    
    assert_eq!(filter.time_range, Some((1000, 2000)));
}

#[test]
fn test_event_filter_with_event_types() {
    let types = vec!["Transfer".to_string(), "Approval".to_string()];
    let filter = EventFilter::new()
        .with_event_types(types.clone());
    
    assert_eq!(filter.event_types, Some(types));
}

#[test]
fn test_event_filter_with_addresses() {
    let addresses = vec!["0x1234".to_string(), "0x5678".to_string()];
    let filter = EventFilter::new()
        .with_addresses(addresses.clone());
    
    assert_eq!(filter.addresses, Some(addresses));
}

#[test]
fn test_event_filter_with_tx_hashes() {
    let hashes = vec!["0xabcd".to_string(), "0xef01".to_string()];
    let filter = EventFilter::new()
        .with_tx_hashes(hashes.clone());
    
    assert_eq!(filter.tx_hashes, Some(hashes));
}

#[test]
fn test_event_filter_with_limit() {
    let filter = EventFilter::new()
        .with_limit(100);
    
    assert_eq!(filter.limit, Some(100));
}

#[test]
fn test_event_filter_with_offset() {
    let filter = EventFilter::new()
        .with_offset(50);
    
    assert_eq!(filter.offset, Some(50));
}

#[test]
fn test_event_filter_builder_pattern() {
    let filter = EventFilter::new()
        .with_chain("ethereum")
        .with_block_range(1000, 2000)
        .with_event_types(vec!["Transfer".to_string()])
        .with_limit(50)
        .with_offset(10);
    
    assert_eq!(filter.chain, Some("ethereum".to_string()));
    assert_eq!(filter.block_range, Some((1000, 2000)));
    assert_eq!(filter.event_types, Some(vec!["Transfer".to_string()]));
    assert_eq!(filter.limit, Some(50));
    assert_eq!(filter.offset, Some(10));
}

#[test]
fn test_event_filter_matches_chain() {
    let filter = EventFilter::new().with_chain("ethereum");
    
    assert!(filter.matches_chain("ethereum"));
    assert!(!filter.matches_chain("cosmos"));
    
    let no_chain_filter = EventFilter::new();
    assert!(no_chain_filter.matches_chain("ethereum"));
    assert!(no_chain_filter.matches_chain("cosmos"));
}

#[test]
fn test_event_filter_matches_block() {
    let filter = EventFilter::new().with_block_range(100, 200);
    
    assert!(filter.matches_block(150));
    assert!(filter.matches_block(100));
    assert!(filter.matches_block(200));
    assert!(!filter.matches_block(99));
    assert!(!filter.matches_block(201));
    
    let no_block_filter = EventFilter::new();
    assert!(no_block_filter.matches_block(150));
}

#[test]
fn test_event_filter_matches_event_type() {
    let filter = EventFilter::new()
        .with_event_types(vec!["Transfer".to_string(), "Approval".to_string()]);
    
    assert!(filter.matches_event_type("Transfer"));
    assert!(filter.matches_event_type("Approval"));
    assert!(!filter.matches_event_type("Mint"));
    
    let no_type_filter = EventFilter::new();
    assert!(no_type_filter.matches_event_type("Transfer"));
}

#[test]
fn test_event_filter_matches_address() {
    let filter = EventFilter::new()
        .with_addresses(vec!["0x1234".to_string(), "0x5678".to_string()]);
    
    assert!(filter.matches_address("0x1234"));
    assert!(filter.matches_address("0x5678"));
    assert!(!filter.matches_address("0x9abc"));
    
    let no_address_filter = EventFilter::new();
    assert!(no_address_filter.matches_address("0x1234"));
}

#[test]
fn test_event_filter_matches_tx_hash() {
    let filter = EventFilter::new()
        .with_tx_hashes(vec!["0xabcd".to_string(), "0xef01".to_string()]);
    
    assert!(filter.matches_tx_hash("0xabcd"));
    assert!(filter.matches_tx_hash("0xef01"));
    assert!(!filter.matches_tx_hash("0x2345"));
    
    let no_hash_filter = EventFilter::new();
    assert!(no_hash_filter.matches_tx_hash("0xabcd"));
}

#[test]
fn test_event_filter_aliases() {
    let filter = EventFilter::new()
        .chain("ethereum".to_string())
        .block_range(100, 200)
        .event_type("Transfer".to_string())
        .address("0x1234".to_string());
    
    assert_eq!(filter.chain, Some("ethereum".to_string()));
    assert_eq!(filter.block_range, Some((100, 200)));
    assert_eq!(filter.event_types, Some(vec!["Transfer".to_string()]));
    assert_eq!(filter.addresses, Some(vec!["0x1234".to_string()]));
}

#[test]
fn test_event_filter_paginate() {
    let filter = EventFilter::new()
        .paginate(Some(100), Some(50));
    
    assert_eq!(filter.limit, Some(100));
    assert_eq!(filter.offset, Some(50));
    
    let partial_filter = EventFilter::new()
        .paginate(Some(100), None);
    
    assert_eq!(partial_filter.limit, Some(100));
    assert_eq!(partial_filter.offset, None);
}