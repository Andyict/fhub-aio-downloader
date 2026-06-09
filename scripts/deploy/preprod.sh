#!/bin/bash
# FHub Pre-Production Deployment - Build for linux/amd64 and push to GHCR
# Builds Docker image for Proxmox platform only (faster than multi-platform)

set -e

# Colors and Icons
BLUE='\033[0;34m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
GHCR_IMAGE="ghcr.io/nas2nd/fhub"
PLATFORM="linux/amd64"

# Get project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Header
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       🚀 FHub Pre-Prod Deployment 🚀            ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${CYAN}📍 Platform:${NC}  ${PLATFORM} (Proxmox)"
echo -e "${CYAN}🐳 Registry:${NC} GHCR"
echo -e "${CYAN}🏷️  Tag:${NC}      preprod"
echo ""

# Step 0: Check Docker Daemon
echo -e "${YELLOW}[1/5]${NC} 🐳 Checking Docker Desktop..."
if ! docker info >/dev/null 2>&1; then
    echo -e "${YELLOW}      ⚠️  Docker Desktop is not running${NC}"
    echo -e "${CYAN}      🚀 Starting Docker Desktop...${NC}"
    open -a Docker
    
    # Wait for Docker to start (max 60 seconds)
    echo -e "${CYAN}      ⏳ Waiting for Docker to start...${NC}"
    for i in {1..30}; do
        if docker info >/dev/null 2>&1; then
            echo -e "${GREEN}      ✓ Docker Desktop is ready${NC}"
            break
        fi
        if [ $i -eq 30 ]; then
            echo -e "${RED}      ✗ Docker failed to start after 60 seconds${NC}"
            echo -e "${YELLOW}      Please start Docker Desktop manually and try again${NC}"
            exit 1
        fi
        sleep 2
    done
else
    echo -e "${GREEN}      ✓ Docker Desktop is running${NC}"
fi
echo ""

# Step 1: Check GHCR Authentication
echo -e "${YELLOW}[2/5]${NC} 🔐 Verifying GHCR authentication..."
if ! docker info 2>/dev/null | grep -q "ghcr.io"; then
    echo -e "${YELLOW}      ⚠️  Not logged in to GHCR${NC}"
    echo ""
    echo -e "${CYAN}      Please login with:${NC}"
    echo -e "${CYAN}      echo \$GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin${NC}"
    echo ""
    read -p "      Login now? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        read -p "      GitHub username: " GH_USER
        echo -e "${CYAN}      Enter your GitHub Personal Access Token:${NC}"
        read -s GH_TOKEN
        echo
        echo "$GH_TOKEN" | docker login ghcr.io -u "$GH_USER" --password-stdin
        echo -e "${GREEN}      ✓ Logged in successfully${NC}"
    else
        echo -e "${RED}      ✗ Cannot proceed without authentication${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}      ✓ Already authenticated${NC}"
fi
echo ""

# Step 2: Get version info
cd "$PROJECT_ROOT"
VERSION=$(git describe --tags --always 2>/dev/null || echo "dev")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
VCS_REF=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo -e "${YELLOW}[3/5]${NC} 📋 Build Information:"
echo -e "${CYAN}      📌 Version: ${VERSION}${NC}"
echo -e "${CYAN}      📅 Build Date: ${BUILD_DATE}${NC}"
echo -e "${CYAN}      🔖 Git Commit: ${VCS_REF}${NC}"
echo -e "${CYAN}      🖥️  Platform: ${PLATFORM}${NC}"
echo ""

# Step 3: Build for linux/amd64 only
echo -e "${YELLOW}[4/5]${NC} 🏗️  Building Docker image for ${PLATFORM}..."
echo -e "${CYAN}      This builds only for Proxmox (faster than multi-platform)${NC}"

docker buildx build \
    --platform "${PLATFORM}" \
    --build-arg VERSION="${VERSION}" \
    --build-arg BUILD_DATE="${BUILD_DATE}" \
    --build-arg VCS_REF="${VCS_REF}" \
    -t "${GHCR_IMAGE}:preprod" \
    -f Dockerfile \
    --load \
    . > debug_log/preprod-build.log 2>&1

if [ $? -eq 0 ]; then
    echo -e "${GREEN}      ✓ Docker image built successfully${NC}"
else
    echo -e "${RED}      ✗ Docker build failed - check debug_log/preprod-build.log${NC}"
    tail -50 debug_log/preprod-build.log
    exit 1
fi
echo ""

# Step 4: Push to GHCR
echo -e "${YELLOW}[5/5]${NC} 📤 Pushing to GHCR..."
echo -e "${CYAN}      Pushing ${GHCR_IMAGE}:preprod...${NC}"

if docker push "${GHCR_IMAGE}:preprod" 2>&1 | grep -E "Pushed|digest:"; then
    echo -e "${GREEN}      ✓ Pushed successfully${NC}"
else
    echo -e "${RED}      ✗ Failed to push${NC}"
    exit 1
fi
echo ""

# Summary
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Pre-production build complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}📦 Published Image:${NC}"
echo -e "${CYAN}   ${GHCR_IMAGE}:preprod${NC}"
echo ""
echo -e "${CYAN}🔗 View on GitHub:${NC}"
echo -e "${CYAN}   https://github.com/nas2nd/fhub/pkgs/container/fhub${NC}"
echo ""
echo -e "${CYAN}📥 Pull Command:${NC}"
echo -e "${CYAN}   docker pull ${GHCR_IMAGE}:preprod${NC}"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo -e "${CYAN}   Deploy to production: ./production.sh${NC}"
echo -e "${CYAN}   (Update production.sh to use :preprod tag)${NC}"
echo ""
