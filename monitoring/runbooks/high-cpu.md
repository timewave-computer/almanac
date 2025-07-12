# High CPU Usage Runbook

## Alert: HighCPUUsage

### Overview
This alert triggers when CPU usage exceeds 80% for more than 5 minutes.

### Impact
- Degraded performance
- Increased response times
- Potential service instability

### Diagnosis Steps

1. **Check current CPU usage**
   ```bash
   # Check system CPU usage
   top -b -n 1 | head -20
   
   # Check process-specific CPU usage
   ps aux | sort -nrk 3,3 | head -10
   ```

2. **Identify CPU-intensive operations**
   ```bash
   # Check event processing rate
   curl -s http://localhost:9090/metrics | grep almanac_events_per_second
   
   # Check active goroutines
   curl -s http://localhost:9090/metrics | grep almanac_goroutines_count
   ```

3. **Review logs for errors**
   ```bash
   # Check for processing errors
   tail -f /var/log/almanac/almanac.log | grep ERROR
   
   # Look for panic or deadlock
   journalctl -u almanac-indexer -n 100 --no-pager
   ```

### Mitigation Steps

1. **Immediate Actions**
   - Reduce event processing rate if possible
   - Check for and kill any runaway processes
   - Verify no infinite loops in processing

2. **Scale Horizontally**
   ```bash
   # Add more worker nodes
   almanac-cli node add --type worker --count 2
   
   # Rebalance workload
   almanac-cli rebalance --strategy even
   ```

3. **Optimize Performance**
   - Enable batch processing
   - Increase batch sizes
   - Review and optimize slow queries

### Prevention

1. Set up CPU usage trending dashboards
2. Implement proper rate limiting
3. Regular performance testing
4. Capacity planning based on growth projections

### Escalation

If CPU usage remains high after mitigation:
1. Page on-call engineer
2. Consider emergency scaling
3. Prepare for potential service degradation