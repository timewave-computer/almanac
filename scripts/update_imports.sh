#!/bin/bash
# Import update script for Almanac reorganization

set -e

echo "🔄 Updating import paths..."

# Function to update imports in a file
update_imports() {
    local file=$1
    if [ -f "$file" ]; then
        echo "  Updating imports in $file"
        
        # Update distributed imports
        sed -i '' 's/use crate::distributed_/use indexer_distributed::/g' "$file"
        sed -i '' 's/use indexer_core::distributed_/use indexer_distributed::/g' "$file"
        sed -i '' 's/use super::distributed_/use indexer_distributed::/g' "$file"
        
        # Update pipeline imports
        sed -i '' 's/use crate::pipeline/use indexer_pipeline::pipeline/g' "$file"
        sed -i '' 's/use crate::batch_processor/use indexer_pipeline::batch_processor/g' "$file"
        sed -i '' 's/use crate::event_ordering/use indexer_pipeline::event_ordering/g' "$file"
        sed -i '' 's/use crate::finality_processor/use indexer_pipeline::finality_processor/g' "$file"
        sed -i '' 's/use crate::ingestion_queue/use indexer_pipeline::ingestion_queue/g' "$file"
        sed -i '' 's/use crate::reorg_handler/use indexer_pipeline::reorg_handler/g' "$file"
        sed -i '' 's/use crate::sync_tracker/use indexer_pipeline::sync_tracker/g' "$file"
        
        # Update observability imports
        sed -i '' 's/use crate::observability/use indexer_observability::observability/g' "$file"
        sed -i '' 's/use crate::metrics/use indexer_observability::metrics/g' "$file"
        sed -i '' 's/use crate::health/use indexer_observability::health/g' "$file"
        sed -i '' 's/use crate::logging/use indexer_observability::logging/g' "$file"
        
        # Update validation imports
        sed -i '' 's/use crate::validation/use indexer_validation::validation/g' "$file"
        sed -i '' 's/use crate::semantic_validation/use indexer_validation::semantic_validation/g' "$file"
        
        # Update storage imports
        sed -i '' 's/use indexer_storage::/use indexer_storage_core::/g' "$file"
        sed -i '' 's/use crate::postgres/use indexer_storage_postgres/g' "$file"
        sed -i '' 's/use crate::rocks/use indexer_storage_rocks/g' "$file"
        sed -i '' 's/use crate::memory/use indexer_storage_memory/g' "$file"
        
        # Update API imports
        sed -i '' 's/use crate::http/use indexer_api_rest::http/g' "$file"
        sed -i '' 's/use crate::graphql/use indexer_api_graphql/g' "$file"
        sed -i '' 's/use crate::websocket/use indexer_api_streaming::websocket/g' "$file"
        sed -i '' 's/use crate::auth/use indexer_api_core::auth/g' "$file"
        
        # Update cross-chain imports
        sed -i '' 's/use crate::cross_chain/use indexer_cross_chain/g' "$file"
        sed -i '' 's/use crate::correlation/use indexer_cross_chain::correlation/g' "$file"
        
        # Update data services imports
        sed -i '' 's/use crate::aggregation/use indexer_data_services::aggregation/g' "$file"
        sed -i '' 's/use crate::archival/use indexer_data_services::archival/g' "$file"
        sed -i '' 's/use crate::backup_restore/use indexer_data_services::backup_restore/g' "$file"
        sed -i '' 's/use crate::caching/use indexer_data_services::caching/g' "$file"
        sed -i '' 's/use crate::compression/use indexer_data_services::compression/g' "$file"
        sed -i '' 's/use crate::data_export/use indexer_data_services::data_export/g' "$file"
        sed -i '' 's/use crate::text_search/use indexer_data_services::text_search/g' "$file"
    fi
}

# Find all Rust files and update imports
echo "🔍 Finding all Rust files..."
find crates -name "*.rs" -type f | while read -r file; do
    update_imports "$file"
done

# Update Cargo.toml dependencies
echo "📦 Updating Cargo.toml dependencies..."

update_cargo_deps() {
    local file=$1
    if [ -f "$file" ]; then
        echo "  Updating $file"
        
        # Add new dependencies
        sed -i '' 's/indexer-core = {/indexer-core = { path = "..\/core" }\nindexer-distributed = { path = "..\/distributed", optional = true }\nindexer-pipeline = { path = "..\/pipeline" }\nindexer-observability = { path = "..\/observability" }\nindexer-validation = { path = "..\/validation" }\n#indexer-core = {/g' "$file"
        
        # Update storage dependencies
        sed -i '' 's/indexer-storage = {/indexer-storage-core = { path = "..\/storage-core" }/g' "$file"
        
        # Update API dependencies
        sed -i '' 's/indexer-api = {/indexer-api-core = { path = "..\/api-core" }/g' "$file"
    fi
}

# Update all Cargo.toml files
find crates -name "Cargo.toml" -type f | while read -r file; do
    update_cargo_deps "$file"
done

echo "✅ Import updates complete!"
echo ""
echo "Next step: Run 'cargo check' to verify all imports are correct."