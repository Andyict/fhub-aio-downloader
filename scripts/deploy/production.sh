#!/bin/bash
# FHub Production Deployment - Deploy from GHCR
# Pulls latest image from GitHub Container Registry and deploys to LXC 112

set -e

# Colors and Icons
BLUE='\033[0;34m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
LXC_ID="112"
LXC_HOST="pve-remote"
GHCR_IMAGE="ghcr.io/nas2nd/fhub:latest"
DEPLOY_DIR="/opt/fhub"
APPDATA_DIR="/mnt/appdata/fhub"
DOWNLOAD_DIR="/data/fhub-download"

# Header
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       🌟 FHub Production Deployment 🌟          ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${CYAN}📍 Target:${NC}    LXC ${LXC_ID} on ${LXC_HOST}"
echo -e "${CYAN}🐳 Image:${NC}     ${GHCR_IMAGE}"
echo -e "${CYAN}📦 Source:${NC}    GitHub Container Registry"
echo ""

# Step 1: Stop and Remove Old Containers
echo -e "${YELLOW}[1/5]${NC} 🛑 Stopping old containers..."
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    cd ${DEPLOY_DIR} 2>/dev/null || true
    docker compose down 2>/dev/null || true
    docker stop fhub 2>/dev/null || true
    docker rm fhub 2>/dev/null || true
    echo \"Old containers stopped\"
'" && echo -e "${GREEN}      ✓ Old containers removed${NC}"
echo ""

# Step 2: Clean Old Images
echo -e "${YELLOW}[2/5]${NC} 🧹 Cleaning old Docker images..."
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    # Remove old fhub images (keep GHCR images)
    docker images | grep \"fhub\" | grep -v \"ghcr.io\" | awk \"{print \\\$3}\" | xargs -r docker rmi -f 2>/dev/null || true
    echo \"Old images cleaned\"
'" && echo -e "${GREEN}      ✓ Cleanup complete${NC}"
echo ""

# Step 3: Pull Latest Image from GHCR
echo -e "${YELLOW}[3/5]${NC} 📥 Pulling latest image from GHCR..."
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    echo \"Pulling ${GHCR_IMAGE}...\"
    if docker pull ${GHCR_IMAGE}; then
        echo \"Image pulled successfully\"
        
        # Show image info
        echo \"\"
        echo \"Image Details:\"
        docker inspect ${GHCR_IMAGE} --format \"Version: {{index .Config.Labels \\\"org.opencontainers.image.version\\\"}}\"
        docker inspect ${GHCR_IMAGE} --format \"Created: {{index .Config.Labels \\\"org.opencontainers.image.created\\\"}}\"
        docker inspect ${GHCR_IMAGE} --format \"Revision: {{index .Config.Labels \\\"org.opencontainers.image.revision\\\"}}\"
        exit 0
    else
        echo \"Failed to pull image\"
        exit 1
    fi
'" && echo -e "${GREEN}      ✓ Image pulled successfully${NC}" || { echo -e "${RED}      ✗ Failed to pull image${NC}"; exit 1; }
echo ""

# Step 4: Create docker-compose.yml
echo -e "${YELLOW}[4/5]${NC} 📝 Creating docker-compose.yml..."
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    mkdir -p ${DEPLOY_DIR}
    cat > ${DEPLOY_DIR}/docker-compose.yml <<EOF
version: \"3.8\"

services:
  fhub:
    image: ${GHCR_IMAGE}
    container_name: fhub
    restart: unless-stopped
    ports:
      - \"8484:8484\"
    volumes:
      - ${APPDATA_DIR}:/appData
      - ${DOWNLOAD_DIR}:/appData/downloads
    environment:
      - FHUB_APPDATA_DIR=/appData
      - RUST_LOG=fhub=info,tower_http=info
      - TZ=Asia/Bangkok
    healthcheck:
      test: [\"CMD\", \"curl\", \"-f\", \"http://localhost:8484/health\"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
EOF
    echo \"docker-compose.yml created\"
'" && echo -e "${GREEN}      ✓ Configuration created${NC}"
echo ""

# Step 5: Start FHub
echo -e "${YELLOW}[5/5]${NC} 🚀 Starting FHub..."
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    cd ${DEPLOY_DIR}
    docker compose up -d
    echo \"FHub started\"
'" && echo -e "${GREEN}      ✓ Container started${NC}" || { echo -e "${RED}      ✗ Failed to start container${NC}"; exit 1; }
echo ""

# Wait for health check
echo -e "${CYAN}⏳ Waiting for health check...${NC}"
sleep 10

# Verify health
echo -e "${CYAN}🔍 Verifying deployment...${NC}"
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    for i in {1..30}; do
        if docker exec fhub curl -f http://localhost:8484/health 2>/dev/null; then
            echo \"\"
            echo \"✓ Health check passed!\"
            exit 0
        fi
        echo -n \".\"
        sleep 2
    done
    echo \"\"
    echo \"⚠ Health check timeout (container may still be starting)\"
    exit 0
'" || true

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Production deployment complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""

# Show status
echo -e "${CYAN}📊 Deployment Status:${NC}"
ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- bash -c '
    echo \"\"
    echo \"Container Status:\"
    docker ps -a | grep fhub
    echo \"\"
    echo \"Image Info:\"
    docker images | grep fhub
'" || true

echo ""
LXC_IP=$(ssh root@${LXC_HOST} "pct exec ${LXC_ID} -- hostname -I | awk '{print \$1}'")
echo -e "${CYAN}🌐 Access:${NC}    http://${LXC_IP}:8484"
echo -e "${CYAN}📋 Logs:${NC}     ssh root@${LXC_HOST} \"pct exec ${LXC_ID} -- docker logs -f fhub\""
echo -e "${CYAN}🔍 Status:${NC}   ssh root@${LXC_HOST} \"pct exec ${LXC_ID} -- docker ps\""
echo -e "${CYAN}🔄 Restart:${NC}  ssh root@${LXC_HOST} \"pct exec ${LXC_ID} -- docker restart fhub\""
echo ""
