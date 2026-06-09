#!/bin/bash
# FHub Development Mode - Local Debug Deployment
# Runs backend and frontend with hot-reload for rapid development

set -e

# Colors and Icons
BLUE='\033[0;34m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m'

# Get project root (2 levels up from scripts/deploy/)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Header
echo ""
echo -e "${BLUE}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         🛠️  FHub Development Mode 🛠️              ║${NC}"
echo -e "${BLUE}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${CYAN}📍 Mode:${NC}      Debug (Hot-Reload Enabled)"
echo -e "${CYAN}🌐 Frontend:${NC}  http://localhost:5173"
echo -e "${CYAN}🔧 Backend:${NC}   http://localhost:8484"
echo ""

# Step 1: Create debug_log directory and Cleanup
echo -e "${YELLOW}[1/4]${NC} 🧹 Cleaning up previous sessions..."
mkdir -p "$PROJECT_ROOT/debug_log"
pkill -f "target/debug/fhub" 2>/dev/null || true
pkill -f "vite" 2>/dev/null || true
sleep 1
echo -e "${GREEN}      ✓ Cleanup complete${NC}"
echo ""

# Step 2: Build Backend
echo -e "${YELLOW}[2/4]${NC} 🦀 Building Rust Backend (Debug Mode)..."
cd "$PROJECT_ROOT/backend"
if cargo build 2>&1 | tee "$PROJECT_ROOT/debug_log/backend-build.log" | grep -q "Finished"; then
    echo -e "${GREEN}      ✓ Backend build successful${NC}"
else
    echo -e "${RED}      ✗ Backend build failed - check debug_log/backend-build.log${NC}"
    exit 1
fi
echo ""

# Step 3: Start Backend
echo -e "${YELLOW}[3/4]${NC} 🚀 Starting Backend Server..."
cd "$PROJECT_ROOT/backend"
cargo run > "$PROJECT_ROOT/debug_log/run.log" 2>&1 &
BACKEND_PID=$!
echo -e "${GREEN}      ✓ Backend started (PID: $BACKEND_PID)${NC}"
echo -e "${CYAN}      📋 Logs: tail -f debug_log/run.log${NC}"
echo ""

# Wait for backend to be ready
echo -e "${YELLOW}[4/4]${NC} ⏳ Waiting for backend to be ready..."
sleep 3

# Check if backend is running
if ps -p $BACKEND_PID > /dev/null; then
    echo -e "${GREEN}      ✓ Backend is running${NC}"
else
    echo -e "${RED}      ✗ Backend failed to start${NC}"
    exit 1
fi
echo ""

# Step 4: Start Frontend
echo -e "${YELLOW}[4/4]${NC} 🎨 Launching Frontend Dev Server..."
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Development environment ready!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}🌐 Frontend:${NC} http://localhost:5173"
echo -e "${CYAN}🔧 Backend:${NC}  http://localhost:8484"
echo -e "${CYAN}📋 Logs:${NC}     tail -f debug_log/run.log"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo ""

cd "$PROJECT_ROOT/frontend"

# Cleanup on exit
trap "echo ''; echo -e '${YELLOW}🛑 Stopping services...${NC}'; kill $BACKEND_PID 2>/dev/null; echo -e '${GREEN}✓ All services stopped${NC}'; exit 0" EXIT INT TERM

npm run dev
