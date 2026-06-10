#!/bin/bash
# FHub Update Script
# Usage: ./update.sh
# Safe update path for Docker Compose installs. Keeps appData/downloads volumes intact.

set -euo pipefail

GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml}"
SERVICE_NAME="${SERVICE_NAME:-fhub}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
STAMP="$(date +%Y%m%d-%H%M%S)"

echo -e "${BLUE}╔═══════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          FHub Update Script           ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════╝${NC}"
echo ""

if ! command -v docker >/dev/null 2>&1; then
  echo -e "${RED}❌ Docker is not installed or not in PATH${NC}"
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo -e "${RED}❌ Docker Compose is not available${NC}"
  exit 1
fi

if [ ! -f "$COMPOSE_FILE" ]; then
  echo -e "${RED}❌ Cannot find $COMPOSE_FILE in $(pwd)${NC}"
  echo -e "${YELLOW}Run this script from your FHub install folder, or set COMPOSE_FILE=/path/to/docker-compose.yml${NC}"
  exit 1
fi

mkdir -p "$BACKUP_DIR"

echo -e "${YELLOW}📦 Backing up compose file...${NC}"
cp "$COMPOSE_FILE" "$BACKUP_DIR/docker-compose.$STAMP.yml"

# Best-effort backup for bind-mounted ./appData installs. Named Docker volumes remain untouched by the update.
if [ -d "./appData" ]; then
  echo -e "${YELLOW}📦 Backing up ./appData metadata...${NC}"
  tar -czf "$BACKUP_DIR/appData.$STAMP.tgz" ./appData
fi

echo -e "${YELLOW}📥 Pulling latest FHub image...${NC}"
docker compose -f "$COMPOSE_FILE" pull "$SERVICE_NAME"

echo -e "${YELLOW}🚀 Recreating FHub container...${NC}"
docker compose -f "$COMPOSE_FILE" up -d "$SERVICE_NAME"

echo -e "${YELLOW}🧹 Removing unused old images...${NC}"
docker image prune -f >/dev/null || true

echo ""
echo -e "${GREEN}✅ FHub update complete.${NC}"
echo -e "${BLUE}Useful checks:${NC}"
echo -e "  docker compose -f $COMPOSE_FILE ps"
echo -e "  docker compose -f $COMPOSE_FILE logs -f $SERVICE_NAME"
