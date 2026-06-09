#!/bin/bash
# Publish FHub to GitHub Container Registry (GHCR)
# Builds and pushes Docker image with proper versioning

set -e

# Colors and Icons
BLUE='\033[0;34m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

# Get project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Header
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║       📦 Publish FHub to GHCR 📦                 ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if logged in to GHCR
echo -e "${YELLOW}[1/5]${NC} 🔐 Verifying GHCR authentication..."
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

# Get version info
cd "$PROJECT_ROOT"

# Try to get version from git tag
VERSION=$(git describe --tags --exact-match 2>/dev/null || git describe --tags --always 2>/dev/null || echo "dev")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
VCS_REF=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

# Check if this is a release tag (vX.Y.Z)
IS_RELEASE=false
if [[ $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    IS_RELEASE=true
    VERSION_CLEAN=${VERSION#v}  # Remove 'v' prefix
    MAJOR=$(echo $VERSION_CLEAN | cut -d. -f1)
    MINOR=$(echo $VERSION_CLEAN | cut -d. -f1-2)
fi

echo -e "${YELLOW}[2/5]${NC} 📋 Version Information:"
echo -e "${CYAN}      📌 Version: ${VERSION}${NC}"
echo -e "${CYAN}      📅 Build Date: ${BUILD_DATE}${NC}"
echo -e "${CYAN}      🔖 Git Commit: ${VCS_REF}${NC}"
echo -e "${CYAN}      🏷️  Release: ${IS_RELEASE}${NC}"
echo ""

# Determine tags to push
TAGS=()
if [ "$IS_RELEASE" = true ]; then
    TAGS+=("ghcr.io/nas2nd/fhub:v${VERSION_CLEAN}")
    TAGS+=("ghcr.io/nas2nd/fhub:v${MINOR}")
    TAGS+=("ghcr.io/nas2nd/fhub:v${MAJOR}")
    TAGS+=("ghcr.io/nas2nd/fhub:stable")
    TAGS+=("ghcr.io/nas2nd/fhub:latest")
else
    TAGS+=("ghcr.io/nas2nd/fhub:${VERSION}")
    TAGS+=("ghcr.io/nas2nd/fhub:nightly")
fi

echo -e "${CYAN}      Tags to be pushed:${NC}"
for tag in "${TAGS[@]}"; do
    echo -e "${CYAN}        - ${tag}${NC}"
done
echo ""

read -p "Continue with build and push? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Cancelled${NC}"
    exit 0
fi
echo ""

# Build image
echo -e "${YELLOW}[3/5]${NC} 🏗️  Building Docker image..."

# Build with first tag
BUILD_ARGS=""
for tag in "${TAGS[@]}"; do
    BUILD_ARGS="$BUILD_ARGS -t $tag"
done

if docker build \
    --build-arg VERSION="${VERSION}" \
    --build-arg BUILD_DATE="${BUILD_DATE}" \
    --build-arg VCS_REF="${VCS_REF}" \
    $BUILD_ARGS \
    -f Dockerfile \
    . 2>&1 | tee debug_log/ghcr-build.log | grep -E "Step|Successfully|ERROR"; then
    echo -e "${GREEN}      ✓ Docker image built successfully${NC}"
else
    echo -e "${RED}      ✗ Docker build failed - check debug_log/ghcr-build.log${NC}"
    exit 1
fi
echo ""

# Push images
echo -e "${YELLOW}[4/5]${NC} 📤 Pushing images to GHCR..."
PUSH_COUNT=0
TOTAL_TAGS=${#TAGS[@]}

for tag in "${TAGS[@]}"; do
    PUSH_COUNT=$((PUSH_COUNT + 1))
    echo -e "${CYAN}      [${PUSH_COUNT}/${TOTAL_TAGS}] Pushing ${tag}...${NC}"
    if docker push "$tag" 2>&1 | grep -E "Pushed|digest:"; then
        echo -e "${GREEN}      ✓ Pushed successfully${NC}"
    else
        echo -e "${RED}      ✗ Failed to push ${tag}${NC}"
    fi
done
echo ""

# Verify
echo -e "${YELLOW}[5/5]${NC} 🔍 Verifying published images..."
echo -e "${CYAN}      Checking latest tag...${NC}"

VERIFY_TAG="${TAGS[0]}"
if docker manifest inspect "$VERIFY_TAG" >/dev/null 2>&1; then
    echo -e "${GREEN}      ✓ Image verified on GHCR${NC}"
    
    # Show image info
    echo ""
    echo -e "${CYAN}      Image Details:${NC}"
    docker inspect "$VERIFY_TAG" --format "        Version: {{index .Config.Labels \"org.opencontainers.image.version\"}}" 2>/dev/null || true
    docker inspect "$VERIFY_TAG" --format "        Created: {{index .Config.Labels \"org.opencontainers.image.created\"}}" 2>/dev/null || true
    docker inspect "$VERIFY_TAG" --format "        Revision: {{index .Config.Labels \"org.opencontainers.image.revision\"}}" 2>/dev/null || true
else
    echo -e "${RED}      ✗ Failed to verify image${NC}"
fi
echo ""

# Summary
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Successfully published to GHCR!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}📦 Published Tags:${NC}"
for tag in "${TAGS[@]}"; do
    echo -e "${CYAN}   - ${tag}${NC}"
done
echo ""
echo -e "${CYAN}🔗 View on GitHub:${NC}"
echo -e "${CYAN}   https://github.com/nas2nd/fhub/pkgs/container/fhub${NC}"
echo ""
echo -e "${CYAN}📥 Pull Command:${NC}"
echo -e "${CYAN}   docker pull ${TAGS[0]}${NC}"
echo ""

if [ "$IS_RELEASE" = true ]; then
    echo -e "${GREEN}🎉 Release ${VERSION} is now available!${NC}"
    echo ""
    echo -e "${YELLOW}Next Steps:${NC}"
    echo -e "${CYAN}   1. Deploy to production: ./scripts/deploy/production.sh${NC}"
    echo -e "${CYAN}   2. Create GitHub release: https://github.com/nas2nd/fhub/releases/new?tag=${VERSION}${NC}"
    echo -e "${CYAN}   3. Announce the release${NC}"
else
    echo -e "${YELLOW}💡 This is a development build${NC}"
    echo -e "${CYAN}   To create a release, use: ./scripts/release.sh${NC}"
fi
echo ""
