#!/bin/bash
# Check current version of running FHub instance

set -e

HOST="${1:-localhost:8484}"

echo "🔍 Checking FHub version..."
echo ""

# Try to get version from API
if command -v curl &> /dev/null; then
    echo "📡 Querying API..."
    VERSION_INFO=$(curl -s "http://${HOST}/health" 2>/dev/null || echo "")
    
    if [ -n "$VERSION_INFO" ]; then
        echo "$VERSION_INFO" | jq . 2>/dev/null || echo "$VERSION_INFO"
    else
        echo "❌ Could not connect to FHub at http://${HOST}"
        exit 1
    fi
fi

echo ""

# If running in Docker, check container labels
if command -v docker &> /dev/null; then
    CONTAINER_ID=$(docker ps --filter "name=fhub" --format "{{.ID}}" 2>/dev/null | head -1)
    
    if [ -n "$CONTAINER_ID" ]; then
        echo "🐳 Docker Container Info:"
        echo ""
        docker inspect "$CONTAINER_ID" --format '
Version:     {{ index .Config.Labels "org.opencontainers.image.version" }}
Build Date:  {{ index .Config.Labels "org.opencontainers.image.created" }}
Git Commit:  {{ index .Config.Labels "org.opencontainers.image.revision" }}
Image:       {{ .Config.Image }}
' 2>/dev/null || echo "No version labels found"
        
        echo ""
        echo "📄 Version files:"
        docker exec "$CONTAINER_ID" cat /app/VERSION 2>/dev/null && echo "" || echo "VERSION file not found"
        docker exec "$CONTAINER_ID" cat /app/BUILD_INFO 2>/dev/null || echo "BUILD_INFO file not found"
    fi
fi
