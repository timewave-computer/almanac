#!/usr/bin/env bash
# persist_wasmd_state.sh - Save and restore wasmd node state for reproducible testing
# Purpose: Provide a way to create snapshots of the wasmd node state and restore them

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WASMD_CONFIG="$PROJECT_ROOT/config/wasmd/config.json"
SNAPSHOT_DIR="$PROJECT_ROOT/build/wasmd-snapshots"

# Define colors for better output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Log function for better output
log() {
  local level=$1
  local message=$2
  
  case $level in
    "info")
      echo -e "${GREEN}[INFO]${NC} $message"
      ;;
    "warn")
      echo -e "${YELLOW}[WARN]${NC} $message"
      ;;
    "error")
      echo -e "${RED}[ERROR]${NC} $message"
      ;;
    *)
      echo "$message"
      ;;
  esac
}

# Parse wasmd config
if [ -f "$WASMD_CONFIG" ]; then
  NODE_HOME=$(jq -r '.node_home' "$WASMD_CONFIG")
else
  log "error" "wasmd configuration not found at $WASMD_CONFIG"
  log "info" "Please run the Valence Contract Integration Setup first:"
  log "info" "  nix run .#valence-contract-integration"
  exit 1
fi

# Check if Nix is available
if ! command -v nix &> /dev/null; then
  log "error" "Nix is not installed. Please install Nix first."
  exit 1
fi

# Ensure snapshot directory exists
mkdir -p "$SNAPSHOT_DIR"

# Function to save wasmd state
save_state() {
  local snapshot_name=$1
  
  if [ -z "$snapshot_name" ]; then
    snapshot_name="snapshot-$(date +%Y%m%d-%H%M%S)"
    log "info" "No snapshot name provided, using: $snapshot_name"
  fi
  
  local snapshot_path="$SNAPSHOT_DIR/$snapshot_name"
  
  # Check if wasmd node is running
  if ! pgrep -f "wasmd start" > /dev/null; then
    log "warn" "wasmd node is not running, saving the current state files"
  else
    log "info" "wasmd node is running, stopping it before saving state"
    pkill -f "wasmd start" || true
    sleep 3
  fi
  
  # Create snapshot directory
  mkdir -p "$snapshot_path"
  
  # Save wasmd state
  log "info" "Saving wasmd state to $snapshot_path"
  cp -a "$NODE_HOME" "$snapshot_path/"
  
  # Create metadata file
  local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
  local metadata="{
    \"name\": \"$snapshot_name\",
    \"created_at\": \"$timestamp\",
    \"description\": \"wasmd state snapshot\",
    \"node_home\": \"$NODE_HOME\"
  }"
  
  echo "$metadata" > "$snapshot_path/metadata.json"
  
  log "info" "wasmd state saved successfully to: $snapshot_path"
}

# Function to restore wasmd state
restore_state() {
  local snapshot_name=$1
  
  if [ -z "$snapshot_name" ]; then
    log "error" "No snapshot name provided"
    log "info" "Available snapshots:"
    list_snapshots
    exit 1
  fi
  
  local snapshot_path="$SNAPSHOT_DIR/$snapshot_name"
  
  # Check if snapshot exists
  if [ ! -d "$snapshot_path" ]; then
    log "error" "Snapshot not found: $snapshot_name"
    log "info" "Available snapshots:"
    list_snapshots
    exit 1
  fi
  
  # Check if wasmd node is running
  if pgrep -f "wasmd start" > /dev/null; then
    log "info" "wasmd node is running, stopping it before restoring state"
    pkill -f "wasmd start" || true
    sleep 3
  fi
  
  # Remove existing wasmd state
  log "info" "Removing existing wasmd state"
  rm -rf "$NODE_HOME"
  
  # Restore wasmd state
  log "info" "Restoring wasmd state from $snapshot_path"
  mkdir -p "$(dirname "$NODE_HOME")"
  cp -a "$snapshot_path/$(basename "$NODE_HOME")" "$(dirname "$NODE_HOME")/"
  
  log "info" "wasmd state restored successfully from: $snapshot_name"
  log "info" "You can now start the wasmd node with: nix run .#wasmd-node"
}

# Function to list available snapshots
list_snapshots() {
  log "info" "Available wasmd snapshots:"
  
  if [ ! "$(ls -A "$SNAPSHOT_DIR" 2>/dev/null)" ]; then
    log "info" "No snapshots found"
    return
  fi
  
  printf "%-25s %-20s %s\n" "NAME" "CREATED AT" "DESCRIPTION"
  printf "%-25s %-20s %s\n" "----" "----------" "-----------"
  
  for snapshot_dir in "$SNAPSHOT_DIR"/*; do
    if [ -d "$snapshot_dir" ] && [ -f "$snapshot_dir/metadata.json" ]; then
      local name=$(jq -r '.name' "$snapshot_dir/metadata.json")
      local created_at=$(jq -r '.created_at' "$snapshot_dir/metadata.json")
      local description=$(jq -r '.description' "$snapshot_dir/metadata.json")
      
      printf "%-25s %-20s %s\n" "$name" "$created_at" "$description"
    fi
  done
}

# Print usage
usage() {
  echo "Usage: $0 [save|restore|list] [snapshot_name]"
  echo ""
  echo "Commands:"
  echo "  save [snapshot_name]    Save current wasmd state to a snapshot"
  echo "  restore <snapshot_name> Restore wasmd state from a snapshot"
  echo "  list                    List available snapshots"
  echo ""
  echo "Examples:"
  echo "  $0 save my-snapshot     Save current state as 'my-snapshot'"
  echo "  $0 restore my-snapshot  Restore state from 'my-snapshot'"
  echo "  $0 list                 List all available snapshots"
}

# Main function
main() {
  local command=$1
  local snapshot_name=$2
  
  case $command in
    save)
      save_state "$snapshot_name"
      ;;
    restore)
      restore_state "$snapshot_name"
      ;;
    list)
      list_snapshots
      ;;
    help)
      usage
      ;;
    *)
      log "error" "Unknown command: $command"
      usage
      exit 1
      ;;
  esac
}

# Run main function
if [ $# -eq 0 ]; then
  usage
  exit 1
fi

main "$@" 