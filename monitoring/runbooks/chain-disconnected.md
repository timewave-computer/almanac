# Chain Disconnected Runbook

## Alert: ChainDisconnected

### Overview
This critical alert triggers when a blockchain connection has been down for 2 minutes.

### Impact
- Missing blockchain events
- Incomplete data indexing
- Potential gaps in event history
- Cross-chain correlation failures

### Diagnosis Steps

1. **Identify disconnected chain**
   ```bash
   # Check chain status
   almanac-cli chain status --detailed
   
   # View connection metrics
   curl -s http://localhost:9090/metrics | grep -E "almanac_chain_connection_status|chain="
   ```

2. **Test chain endpoint**
   ```bash
   # Test RPC endpoint directly
   CHAIN=ethereum  # or cosmos, etc.
   almanac-cli chain test-rpc $CHAIN
   
   # Check endpoint health
   curl -s https://ethereum-rpc.example.com/health
   ```

3. **Review connection logs**
   ```bash
   # Check for connection errors
   grep -i "connection.*failed\|timeout\|refused" /var/log/almanac/almanac.log | tail -50
   
   # Look for rate limiting
   grep -i "rate.*limit\|429\|too.*many" /var/log/almanac/almanac.log | tail -20
   ```

4. **Verify network connectivity**
   ```bash
   # Test DNS resolution
   nslookup ethereum-rpc.example.com
   
   # Test network path
   traceroute ethereum-rpc.example.com
   
   # Check firewall rules
   sudo iptables -L -n | grep -E "REJECT|DROP"
   ```

### Mitigation Steps

1. **Attempt reconnection**
   ```bash
   # Force reconnect
   almanac-cli chain reconnect $CHAIN
   
   # If fails, reset connection pool
   almanac-cli chain reset-connection $CHAIN
   ```

2. **Switch to backup endpoint**
   ```bash
   # List available endpoints
   almanac-cli config get chains.$CHAIN.endpoints
   
   # Switch to backup
   almanac-cli chain switch-endpoint $CHAIN --to backup
   
   # Or add new endpoint
   almanac-cli chain add-endpoint $CHAIN https://backup-rpc.example.com
   ```

3. **Adjust connection parameters**
   ```bash
   # Increase timeout for slow endpoints
   almanac-cli config set chains.$CHAIN.timeout 30s
   
   # Reduce connection pool size if rate limited
   almanac-cli config set chains.$CHAIN.max_connections 5
   
   # Apply configuration
   almanac-cli reload-config
   ```

4. **Handle connection gaps**
   ```bash
   # Identify missing block range
   almanac-cli chain find-gaps $CHAIN
   
   # Backfill missing data
   almanac-cli chain backfill $CHAIN --from-block <start> --to-block <end>
   ```

### Root Cause Analysis

Common causes:
1. RPC endpoint downtime
2. Rate limiting by provider
3. Network connectivity issues
4. SSL/TLS certificate problems
5. Incorrect authentication credentials
6. Firewall or security group changes

### Long-term Solutions

1. **Implement redundancy**
   - Configure multiple RPC endpoints
   - Set up automatic failover
   - Use load balancing across endpoints

2. **Monitor endpoint health**
   ```yaml
   # prometheus alert
   - alert: RPCEndpointUnhealthy
     expr: probe_success{job="blackbox", target=~".*rpc.*"} == 0
     for: 1m
   ```

3. **Establish SLAs**
   - Define uptime requirements with providers
   - Set up dedicated nodes if needed
   - Consider running own full nodes

### Prevention

1. Regular endpoint health checks
2. Maintain list of backup endpoints
3. Monitor rate limit usage
4. Set up alerting for connection issues
5. Document all endpoint configurations

### Escalation

If connection cannot be restored within 10 minutes:
1. Contact RPC provider support
2. Notify stakeholders of potential data gaps
3. Prepare to switch to alternative data sources
4. Consider emergency migration to backup provider