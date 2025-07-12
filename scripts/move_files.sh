#!/bin/bash
# File movement script for Almanac reorganization

set -e

echo "📦 Moving files to new locations..."

# Function to move file if it exists
move_if_exists() {
    if [ -f "$1" ]; then
        echo "  Moving $1 -> $2"
        mkdir -p "$(dirname "$2")"
        mv "$1" "$2"
    fi
}

# Move distributed files
echo "🌐 Moving distributed computing files..."
move_if_exists "crates/core/src/distributed_cache.rs" "crates/distributed/src/cache.rs"
move_if_exists "crates/core/src/distributed_correlation.rs" "crates/distributed/src/correlation.rs"
move_if_exists "crates/core/src/distributed_event_service.rs" "crates/distributed/src/event_service.rs"
move_if_exists "crates/core/src/distributed_monitoring.rs" "crates/distributed/src/monitoring.rs"
move_if_exists "crates/core/src/distributed_query_planner.rs" "crates/distributed/src/query_planner.rs"
move_if_exists "crates/core/src/distributed_query_planner_tests.rs" "crates/distributed/src/query_planner_tests.rs"
move_if_exists "crates/api/src/distributed_gateway.rs" "crates/distributed/src/gateway.rs"
move_if_exists "crates/api/src/distributed_query_api.rs" "crates/distributed/src/query_api.rs"
move_if_exists "crates/storage/src/fdb.rs" "crates/distributed/src/fdb/storage.rs"
move_if_exists "crates/storage/src/fdb_coordination.rs" "crates/distributed/src/fdb/coordination.rs"
move_if_exists "crates/storage/src/fdb_event_correlation.rs" "crates/distributed/src/fdb/event_correlation.rs"
move_if_exists "crates/storage/src/fdb_monitoring.rs" "crates/distributed/src/fdb/monitoring.rs"

# Move pipeline files
echo "🔄 Moving pipeline files..."
move_if_exists "crates/core/src/pipeline.rs" "crates/pipeline/src/pipeline.rs"
move_if_exists "crates/core/src/ingestion_queue.rs" "crates/pipeline/src/ingestion_queue.rs"
move_if_exists "crates/core/src/batch_processor.rs" "crates/pipeline/src/batch_processor.rs"
move_if_exists "crates/core/src/event_ordering.rs" "crates/pipeline/src/event_ordering.rs"
move_if_exists "crates/core/src/finality_processor.rs" "crates/pipeline/src/finality_processor.rs"
move_if_exists "crates/core/src/reorg_handler.rs" "crates/pipeline/src/reorg_handler.rs"
move_if_exists "crates/core/src/sync_tracker.rs" "crates/pipeline/src/sync_tracker.rs"

# Move observability files
echo "📊 Moving observability files..."
move_if_exists "crates/core/src/observability.rs" "crates/observability/src/observability.rs"
move_if_exists "crates/core/src/metrics.rs" "crates/observability/src/metrics.rs"
move_if_exists "crates/core/src/health.rs" "crates/observability/src/health.rs"
move_if_exists "crates/core/src/logging.rs" "crates/observability/src/logging.rs"
move_if_exists "crates/api/src/prometheus_exporter.rs" "crates/observability/src/prometheus_exporter.rs"

# Move validation files
echo "✅ Moving validation files..."
move_if_exists "crates/core/src/validation.rs" "crates/validation/src/validation.rs"
move_if_exists "crates/core/src/semantic_validation.rs" "crates/validation/src/semantic_validation.rs"

# Move storage files
echo "💾 Moving storage files..."
# Storage-postgres
if [ -d "crates/storage/src/postgres" ]; then
    echo "  Moving PostgreSQL storage implementation..."
    mv crates/storage/src/postgres/* crates/storage-postgres/src/ 2>/dev/null || true
fi

# Storage-rocks
move_if_exists "crates/storage/src/rocks.rs" "crates/storage-rocks/src/lib.rs"
move_if_exists "crates/storage/src/optimization.rs" "crates/storage-rocks/src/optimization.rs"
if [ -d "rocksdb_ffi" ]; then
    echo "  Moving RocksDB FFI..."
    mv rocksdb_ffi crates/storage-rocks/
fi

# Storage-memory
move_if_exists "crates/storage/src/memory.rs" "crates/storage-memory/src/lib.rs"

# Move API files
echo "🌐 Moving API files..."
move_if_exists "crates/api/src/rest_enhanced.rs" "crates/api-rest/src/enhanced.rs"
move_if_exists "crates/api/src/openapi.rs" "crates/api-rest/src/openapi.rs"
move_if_exists "crates/api/src/http.rs" "crates/api-rest/src/http.rs"
move_if_exists "crates/api/src/http_enhanced.rs" "crates/api-rest/src/http_enhanced.rs"
move_if_exists "crates/api/src/graphql.rs" "crates/api-graphql/src/lib.rs"
move_if_exists "crates/api/src/subscription.rs" "crates/api-graphql/src/subscription.rs"
move_if_exists "crates/api/src/websocket.rs" "crates/api-streaming/src/websocket.rs"
move_if_exists "crates/api/src/streaming_query_api.rs" "crates/api-streaming/src/query_api.rs"
move_if_exists "crates/api/src/auth.rs" "crates/api-core/src/auth.rs"
move_if_exists "crates/api/src/storage_proof_api.rs" "crates/api-core/src/storage_proof.rs"

# Move cross-chain files
echo "🔗 Moving cross-chain files..."
move_if_exists "crates/core/src/cross_chain.rs" "crates/cross-chain/src/lib.rs"
move_if_exists "crates/core/src/correlation.rs" "crates/cross-chain/src/correlation.rs"

# Move data services files
echo "📈 Moving data services files..."
move_if_exists "crates/core/src/aggregation.rs" "crates/data-services/src/aggregation.rs"
move_if_exists "crates/core/src/archival.rs" "crates/data-services/src/archival.rs"
move_if_exists "crates/core/src/backup_restore.rs" "crates/data-services/src/backup_restore.rs"
move_if_exists "crates/core/src/caching.rs" "crates/data-services/src/caching.rs"
move_if_exists "crates/core/src/compression.rs" "crates/data-services/src/compression.rs"
move_if_exists "crates/core/src/data_export.rs" "crates/data-services/src/data_export.rs"
move_if_exists "crates/core/src/text_search.rs" "crates/data-services/src/text_search.rs"

# Move test files
echo "🧪 Consolidating test files..."
if [ -d "crates/benchmarks/src" ]; then
    mkdir -p crates/testing/src/benchmarks
    mv crates/benchmarks/src/* crates/testing/src/benchmarks/ 2>/dev/null || true
fi

move_if_exists "crates/testing/src/distributed.rs" "crates/testing/src/integration/distributed.rs"
move_if_exists "crates/testing/src/cross_chain_tests.rs" "crates/testing/src/integration/cross_chain.rs"
move_if_exists "crates/testing/src/e2e_tests.rs" "crates/testing/src/e2e/full_pipeline.rs"

echo "✅ File movement complete!"
echo ""
echo "Note: Some files may not have been moved if they didn't exist."
echo "Run 'cargo check' to identify any missing dependencies."