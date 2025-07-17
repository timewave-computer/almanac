#!/bin/bash
# Update imports after infrastructure reorganization

set -e

echo "Updating imports in infrastructure crates..."

# Update storage crate imports
find crates/infrastructure/storage/src -name "*.rs" -type f -exec sed -i.bak \
    -e 's/use indexer_core::/use almanac_infrastructure_core::/g' \
    -e 's/use indexer_storage::/use almanac_infrastructure_storage::/g' \
    {} \;



# Update query module imports
find crates/infrastructure/query/src -name "*.rs" -type f -exec sed -i.bak \
    -e 's/indexer_core::/almanac_infrastructure_core::/g' \
    {} \;

# Update monitoring module imports
find crates/infrastructure/monitoring/src -name "*.rs" -type f -exec sed -i.bak \
    -e 's/indexer_core::/almanac_infrastructure_core::/g' \
    {} \;

# Update caching module imports
find crates/infrastructure/caching/src -name "*.rs" -type f -exec sed -i.bak \
    -e 's/indexer_core::/almanac_infrastructure_core::/g' \
    {} \;

# Update services imports
find crates/services -name "*.rs" -type f -exec sed -i.bak \
    -e 's/use indexer_core::/use almanac_infrastructure_core::/g' \
    -e 's/use indexer_storage::/use almanac_infrastructure_storage::/g' \
    
    {} \;

# Clean up backup files
find crates -name "*.rs.bak" -type f -delete

echo "Import updates complete!"