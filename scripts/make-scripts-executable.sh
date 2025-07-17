#!/bin/bash
# Purpose: Make all scripts in the scripts directory executable

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Making Scripts Executable ===${NC}"

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Check if we're in the scripts directory
if [ ! -d "$SCRIPT_DIR" ]; then
    echo -e "${RED}Error: Cannot find scripts directory${NC}"
    exit 1
fi

cd "$SCRIPT_DIR"

# Make all .sh files executable
echo -e "${BLUE}Finding all shell scripts...${NC}"
count=0

find . -name "*.sh" -type f | while read -r script; do
    if [ ! -x "$script" ]; then
        chmod +x "$script"
        echo -e "${GREEN}✓${NC} Made executable: $script"
        ((count++))
    fi
done

# Also check for scripts without .sh extension
for script in $(find . -type f -exec grep -l '^#!/' {} \; 2>/dev/null | grep -v ".sh$"); do
    if [ ! -x "$script" ]; then
        chmod +x "$script"
        echo -e "${GREEN}✓${NC} Made executable: $script"
        ((count++))
    fi
done

echo -e "${GREEN}✓ All scripts are now executable${NC}"
echo -e "${BLUE}You can run scripts from the project root using ./scripts/script-name.sh${NC}"