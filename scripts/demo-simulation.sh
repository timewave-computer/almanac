#!/bin/bash
# Demo script showing how to use the Almanac simulation environment

set -e

echo "=== Almanac Simulation Environment Demo ==="
echo

# Check if we're in a nix environment
if ! command -v nix &> /dev/null; then
    echo "Error: nix is not available. Please install nix first."
    exit 1
fi

echo "1. Generating simulation configuration..."
echo "   Command: nix run .#sim-config"
nix run .#sim-config

echo
echo "2. Consolidated simulation configuration created:"
ls -la config/simulation.toml

echo
echo "3. Simulation configuration contents:"
cat config/simulation.toml

echo
echo "=== Available Commands ==="
echo "- nix run .#sim-cluster init [chains]  # Initialize with specific chains"
echo "- nix run .#sim-cluster start          # Start all services"
echo "- nix run .#sim-cluster stop           # Stop all services"
echo "- nix run .#sim-cluster status         # Check service status"
echo "- nix run .#sim-cluster logs api       # View API logs"
echo "- nix run .#sim-cluster logs indexer   # View indexer logs"
echo "- nix run .#sim-cluster logs ethereum  # View Ethereum logs"
echo "- nix run .#sim-cluster logs cosmos    # View Cosmos logs"
echo

echo "=== Individual Service Commands ==="
echo "- nix run .#sim-ethereum start         # Start Ethereum node only"
echo "- nix run .#sim-cosmos start           # Start Cosmos node only"
echo "- nix run .#sim-almanac-api start      # Start Almanac API only"
echo "- nix run .#sim-almanac-indexer start  # Start indexer only"
echo

echo "=== Quick Start ==="
echo "1. Run: nix run .#sim-cluster init ethereum,fdb"
echo "2. Run: nix run .#sim-cluster start"
echo "3. Wait for services to start (check with: nix run .#sim-cluster status)"
echo "4. Access Almanac API at: http://localhost:3000"
echo "5. Stop with: nix run .#sim-cluster stop"
echo

echo "Demo complete! The simulation environment is ready to use."