#!/bin/bash
# Generate new validator seeds for devnet
# Usage: ./scripts/generate-devnet-seeds.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEVNET_DIR="$PROJECT_ROOT/devnet"

echo "Generating new validator seeds for devnet..."
echo ""

# Generate seeds using the call-core binary or docker
if command -v docker &> /dev/null && docker images callchain/call-core:latest --format "{{.Repository}}" 2>/dev/null | grep -q "callchain/call-core"; then
    echo "Using Docker to generate seeds..."

    seeds=()
    for i in 1 2 3; do
        seed=$(docker run --rm callchain/call-core:latest generate-seed 2>/dev/null | grep "Seed:" | awk '{print $2}')
        seeds+=("$seed")
        echo "  Node $i seed: $seed"
    done
else
    echo "Docker image not found. Building..."
    "$SCRIPT_DIR/docker-build.sh"

    seeds=()
    for i in 1 2 3; do
        seed=$(docker run --rm callchain/call-core:latest generate-seed 2>/dev/null | grep "Seed:" | awk '{print $2}')
        seeds+=("$seed")
        echo "  Node $i seed: $seed"
    done
fi

echo ""
echo "Updating configuration files..."

# Update node configs with new seeds
for i in 1 2 3; do
    sed -i "s/^validation_seed = .*/validation_seed = \"${seeds[$((i-1))]}\"/" "$DEVNET_DIR/node$i/config.toml"
done

echo ""
echo "Seeds updated in configuration files."
echo ""
echo "IMPORTANT: Save these seeds securely!"
echo "  Node 1: ${seeds[0]}"
echo "  Node 2: ${seeds[1]}"
echo "  Node 3: ${seeds[2]}"
