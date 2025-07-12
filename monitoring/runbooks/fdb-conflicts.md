# FDB High Conflict Rate Runbook

## Alert: FDBHighConflictRate

### Overview
This alert triggers when FoundationDB conflict retries exceed 100/second for 5 minutes.

### Impact
- Increased write latency
- Reduced throughput
- Potential transaction failures
- CPU overhead from retries

### Diagnosis Steps

1. **Check conflict metrics**
   ```bash
   # Get current conflict rate
   curl -s http://localhost:9090/metrics | grep fdb_conflict_retries_total
   
   # Check FDB status
   fdbcli --exec "status details" | grep -A 10 "Conflicted"
   ```

2. **Identify hot keys**
   ```bash
   # Use FDB tooling to find hot ranges
   fdbcli --exec "hotrange list"
   
   # Check transaction logs
   grep -i "conflict\|retry" /var/log/fdb/trace.*.log | tail -100
   ```

3. **Analyze write patterns**
   ```bash
   # Check write distribution
   almanac-cli fdb analyze-writes --duration 5m
   
   # Look for concurrent updates to same keys
   almanac-cli fdb trace --conflicts-only
   ```

4. **Review application logs**
   ```bash
   # Check for batch processing issues
   grep -E "batch.*failed|transaction.*conflict" /var/log/almanac/almanac.log
   
   # Look for specific conflict patterns
   almanac-cli logs analyze --pattern "conflict" --group-by key
   ```

### Mitigation Steps

1. **Reduce conflict rate**
   ```bash
   # Enable conflict avoidance
   almanac-cli config set fdb.conflict_avoidance true
   
   # Increase retry backoff
   almanac-cli config set fdb.retry_backoff_ms 100
   
   # Reduce transaction size
   almanac-cli config set fdb.max_transaction_size 1000
   ```

2. **Optimize write patterns**
   ```bash
   # Enable write batching
   almanac-cli config set processing.batch_writes true
   almanac-cli config set processing.batch_size 100
   
   # Use atomic operations where possible
   almanac-cli config set fdb.use_atomic_ops true
   ```

3. **Redistribute load**
   ```bash
   # Shard hot keys
   almanac-cli fdb shard-keys --pattern "hot_prefix*"
   
   # Enable key prefix randomization
   almanac-cli config set fdb.randomize_prefixes true
   ```

4. **Scale FDB cluster**
   ```bash
   # Add more storage processes
   fdbcli --exec "configure storage=10"
   
   # Increase resolver count
   fdbcli --exec "configure resolvers=5"
   
   # Add more proxies for better load distribution
   fdbcli --exec "configure proxies=5"
   ```

### Advanced Troubleshooting

1. **Transaction analysis**
   ```python
   # Script to analyze transaction patterns
   import fdb
   fdb.api_version(630)
   
   db = fdb.open()
   
   @fdb.transactional
   def analyze_conflicts(tr):
       # Get conflict ranges
       conflicts = tr.get_range_startswith(b'\xff\xff/transaction/conflicted_keys/')
       for k, v in conflicts:
           print(f"Conflicted key: {k}")
   ```

2. **Implement read-before-write optimization**
   ```bash
   # Enable snapshot reads
   almanac-cli config set fdb.use_snapshot_reads true
   
   # Implement optimistic concurrency
   almanac-cli config set fdb.optimistic_locking true
   ```

### Root Cause Analysis

Common causes:
1. Multiple writers updating same keys
2. Large transactions spanning many keys
3. Inefficient key design (hot spots)
4. Insufficient FDB cluster resources
5. Network latency causing longer transaction times

### Long-term Solutions

1. **Redesign data model**
   - Use time-based key prefixes
   - Implement proper sharding strategy
   - Avoid global counters

2. **Implement write coalescing**
   - Batch similar updates
   - Use background aggregation
   - Implement write-ahead logging

3. **Monitor and capacity plan**
   ```yaml
   # Add monitoring for conflict patterns
   - record: fdb_conflict_rate_5m
     expr: rate(fdb_conflict_retries_total[5m])
   
   - alert: FDBConflictTrend
     expr: predict_linear(fdb_conflict_rate_5m[1h], 3600) > 200
   ```

### Prevention

1. Load test with realistic conflict scenarios
2. Review data model for hot spots
3. Implement proper transaction boundaries
4. Use FDB's atomic operations
5. Regular cluster performance tuning

### Escalation

If conflicts persist after mitigation:
1. Engage FDB expertise for cluster tuning
2. Consider emergency sharding of hot data
3. Implement write throttling if necessary
4. Plan data model refactoring sprint