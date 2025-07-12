# Event Processing Stopped Runbook

## Alert: EventProcessingStopped

### Overview
This critical alert triggers when no events have been processed for 5 minutes.

### Impact
- Complete data ingestion failure
- Growing backlog of unprocessed events
- Data freshness SLA violations
- Potential data loss if queues overflow

### Diagnosis Steps

1. **Verify event processing status**
   ```bash
   # Check processing metrics
   curl -s http://localhost:9090/metrics | grep -E "almanac_events_processed_total|almanac_queue_depth"
   
   # Check component health
   curl -s http://localhost:8080/health | jq .
   ```

2. **Check chain connections**
   ```bash
   # Verify chain connectivity
   almanac-cli chain status --all
   
   # Check chain-specific metrics
   curl -s http://localhost:9090/metrics | grep almanac_chain_connection_status
   ```

3. **Investigate storage layer**
   ```bash
   # Check FDB cluster status
   fdbcli --exec "status details"
   
   # Verify storage operations
   curl -s http://localhost:9090/metrics | grep almanac_storage_operations_total
   ```

4. **Review error logs**
   ```bash
   # Check for fatal errors
   tail -n 1000 /var/log/almanac/almanac.log | grep -E "FATAL|PANIC|ERROR"
   
   # Check for OOM kills
   dmesg | grep -i "killed process"
   ```

### Mitigation Steps

1. **Restart event processor**
   ```bash
   # Graceful restart
   almanac-cli component restart event-processor --graceful
   
   # If graceful fails, force restart
   systemctl restart almanac-event-processor
   ```

2. **Clear processing blockages**
   ```bash
   # Check and clear dead letter queue
   almanac-cli queue inspect --dead-letter
   almanac-cli queue clear --dead-letter --confirm
   
   # Reset consumer offsets if needed
   almanac-cli consumer reset --to-latest
   ```

3. **Restore chain connections**
   ```bash
   # Reconnect to chains
   almanac-cli chain reconnect --all
   
   # Verify endpoints are accessible
   for chain in ethereum cosmos; do
     almanac-cli chain test-connection $chain
   done
   ```

4. **Emergency recovery**
   ```bash
   # Start in recovery mode
   almanac-cli start --recovery-mode
   
   # Process backlog with increased resources
   almanac-cli process-backlog --workers 10 --batch-size 1000
   ```

### Root Cause Analysis

Common causes:
1. Chain RPC endpoint failures
2. Storage layer unavailability
3. Memory exhaustion
4. Deadlocks in processing logic
5. Configuration errors after deployment

### Prevention

1. Implement circuit breakers for chain connections
2. Set up automatic failover for RPC endpoints
3. Monitor queue depths and set alerts before overflow
4. Regular health checks for all components
5. Automated recovery procedures

### Escalation

If processing doesn't resume within 15 minutes:
1. Page senior engineer immediately
2. Prepare incident communication
3. Consider switching to backup indexer
4. Document all events in incident timeline