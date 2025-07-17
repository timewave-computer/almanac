# Almanac Monitoring Setup

This directory contains the complete monitoring infrastructure for the Almanac distributed indexer system.

## Overview

The monitoring stack includes:
- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards
- **Alertmanager**: Alert routing and notifications
- **Node Exporter**: System metrics
- **Blackbox Exporter**: Endpoint monitoring
- **Custom Exporters**: FDB and Almanac-specific metrics

## Quick Start

1. **Start the monitoring stack**:
   ```bash
   nix develop --command start-monitoring
   ```

2. **Access the services**:
   - Grafana: http://localhost:3000 (admin/almanac123)
   - Prometheus: http://localhost:9090
   - Alertmanager: http://localhost:9093

3. **Import dashboards**:
   - Dashboards are automatically provisioned from the `dashboards/` directory
   - Available dashboards:
     - Almanac Overview
     - Chain Monitoring
     - FDB Storage Monitoring

## Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌──────────────┐
│  Almanac Node   │────▶│  Prometheus  │────▶│   Grafana    │
│  /metrics:8090  │     │    :9090     │     │    :3000     │
└─────────────────┘     └──────────────┘     └──────────────┘
                               │
                               ▼
                        ┌──────────────┐
                        │ Alertmanager │
                        │    :9093     │
                        └──────────────┘
                               │
                        ┌──────┴───────┐
                        ▼              ▼
                    ┌────────┐    ┌────────┐
                    │ Slack  │    │  Email │
                    └────────┘    └────────┘
```

## Metrics Exposed

### Core Metrics
- `almanac_events_processed_total`: Total events processed
- `almanac_events_failed_total`: Failed event processing
- `almanac_event_processing_duration_seconds`: Processing latency histogram
- `almanac_queue_depth`: Current queue depth
- `almanac_storage_operations_total`: Storage operation counts
- `almanac_http_requests_total`: HTTP request counts
- `almanac_component_health_status`: Component health (0/1)

### Chain Metrics
- `almanac_chain_latest_block`: Latest processed block per chain
- `almanac_chain_blocks_behind_head`: Sync lag
- `almanac_chain_events_processed_total`: Events per chain
- `almanac_chain_reorg_events_total`: Reorg count
- `almanac_chain_connection_status`: Connection health

### FDB Metrics
- `fdb_read_operations_total`: Read operation count
- `fdb_write_operations_total`: Write operation count
- `fdb_operation_latency_seconds`: Operation latency
- `fdb_storage_used_bytes`: Storage usage
- `fdb_conflict_retries_total`: Transaction conflicts
- `fdb_cluster_machines`: Cluster size

## Alert Configuration

Alerts are defined in `alerts/almanac-alerts.yml` and include:

### Critical Alerts
- Event processing stopped
- Component unhealthy
- Chain disconnected
- Critical CPU/Memory usage
- FDB process down

### Warning Alerts
- High CPU/Memory usage
- Event processing slowdown
- High storage error rate
- Chain sync falling behind
- High FDB conflict rate

## Runbooks

Detailed runbooks are provided in the `runbooks/` directory:
- [High CPU Usage](runbooks/high-cpu.md)
- [Event Processing Stopped](runbooks/event-processing-stopped.md)
- [Chain Disconnected](runbooks/chain-disconnected.md)
- [FDB Conflicts](runbooks/fdb-conflicts.md)

## Adding Custom Metrics

1. **In your code**, use the metrics registry:
   ```rust
   let registry = MetricsRegistry::new("mycomponent".to_string());
   registry.register_counter("operations_total", "Total operations", None).await?;
   registry.increment_counter("operations_total", 1.0).await?;
   ```

2. **Register with Prometheus exporter**:
   ```rust
   register_custom_registry(&exporter_state, "mycomponent", registry).await?;
   ```

3. **Access metrics at**: `http://localhost:8090/metrics/custom/mycomponent`

## Performance Tuning

### Prometheus
- Adjust retention: `--storage.tsdb.retention.time=30d`
- Increase memory: Set `PROMETHEUS_MEMORY_LIMIT` env var
- Tune scrape intervals in `prometheus.yml`

### Grafana
- Enable caching for dashboard queries
- Set appropriate refresh intervals
- Use recording rules for expensive queries

### Alertmanager
- Tune grouping to reduce noise
- Set appropriate `group_wait` and `repeat_interval`
- Use inhibition rules to prevent alert storms

## Backup and Recovery

1. **Prometheus data**:
   ```bash
   nix develop --command backup-prometheus
   ```

2. **Grafana dashboards**:
   ```bash
   # Dashboards are in version control at dashboards/
   git add dashboards/*.json
   git commit -m "Backup dashboards"
   ```

3. **Restore from backup**:
   ```bash
   nix develop --command restore-prometheus --backup-path ./backups/snapshot-xxx
   ```

## Troubleshooting

### Prometheus not scraping
1. Check target health: http://localhost:9090/targets
2. Verify network connectivity
3. Check authentication if configured

### Missing metrics
1. Verify exporter is running: `curl http://localhost:8090/metrics`
2. Check metric names and labels
3. Look for errors in Prometheus logs

### Alerts not firing
1. Check alert rules: http://localhost:9090/alerts
2. Verify Alertmanager is receiving alerts
3. Check notification channel configuration

## Security Considerations

1. **Change default passwords** in production
2. **Enable authentication** for all services
3. **Use TLS** for metric endpoints
4. **Restrict network access** to monitoring services
5. **Audit alert notifications** for sensitive data

## Maintenance

### Daily
- Check dashboard health
- Review fired alerts
- Monitor disk usage

### Weekly
- Review metric cardinality
- Check for unused metrics
- Update alert thresholds

### Monthly
- Review and update dashboards
- Audit alerting rules
- Performance optimization
- Backup critical data