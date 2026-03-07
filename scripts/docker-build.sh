#!/bin/bash
# Build Docker image for call-core
# Usage: ./scripts/docker-build.sh [OPTIONS]
#
# Options:
#   --tag, -t TAG     Image tag (default: callchain/call-core:latest)
#   --platform, -p    Target platform (default: linux/amd64)
#   --no-cache        Build without cache
#   --help, -h        Show this help message

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default values
TAG="callchain/call-core:latest"
PLATFORM="linux/amd64"
NO_CACHE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--tag)
            TAG="$2"
            shift 2
            ;;
        -p|--platform)
            PLATFORM="$2"
            shift 2
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        -h|--help)
            echo "Build Docker image for call-core"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -t, --tag TAG       Image tag (default: callchain/call-core:latest)"
            echo "  -p, --platform      Target platform (default: linux/amd64)"
            echo "  --no-cache          Build without cache"
            echo "  -h, --help          Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "Building Docker image..."
echo "  Tag:      $TAG"
echo "  Platform: $PLATFORM"
echo "  Context:  $PROJECT_ROOT"
echo "  BuildKit: enabled (for cache mounts)"

# Enable BuildKit for cache mount support
export DOCKER_BUILDKIT=1

docker build $NO_CACHE \
    --platform "$PLATFORM" \
    --tag "$TAG" \
    "$PROJECT_ROOT"

echo ""
echo "Docker image built successfully!"
echo "  Image: $TAG"
echo ""
echo "To run a single node:"
echo "  docker run -d --name call-node -p 5005:5005 -p 51235:51235 $TAG"
echo ""
echo "To start a 3-node dev testnet:"
echo "  $SCRIPT_DIR/devnet-up.sh"
