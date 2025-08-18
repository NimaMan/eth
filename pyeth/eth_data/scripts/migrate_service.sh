#!/bin/bash

# Migration script from old eth-block-processor to new eth-live-block-processor service
# This script safely migrates the running service to use the new eth_data package

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Service names
OLD_SERVICE="eth-block-processor"
NEW_SERVICE="eth-live-block-processor"

# Paths
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
NEW_SERVICE_FILE="${SCRIPT_DIR}/systemd/${NEW_SERVICE}.service"
OLD_SERVICE_FILE="/etc/systemd/system/${OLD_SERVICE}.service"
NEW_DEST_FILE="/etc/systemd/system/${NEW_SERVICE}.service"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[i]${NC} $1"
}

# Check if running with sudo
if [ "$EUID" -ne 0 ]; then 
    print_error "This script must be run with sudo"
    exit 1
fi

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Service Migration Tool${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "This will migrate from:"
echo "  OLD: ${OLD_SERVICE} (eth_block_processor)"
echo "  NEW: ${NEW_SERVICE} (eth_data)"
echo ""

# Check current state
print_info "Checking current service state..."

OLD_RUNNING=false
if systemctl is-active --quiet ${OLD_SERVICE}.service; then
    OLD_RUNNING=true
    print_warning "${OLD_SERVICE} is currently RUNNING"
else
    print_info "${OLD_SERVICE} is not running"
fi

OLD_ENABLED=false
if systemctl is-enabled --quiet ${OLD_SERVICE}.service 2>/dev/null; then
    OLD_ENABLED=true
    print_info "${OLD_SERVICE} is enabled for auto-start"
fi

# Check if new service already exists
if [ -f "${NEW_DEST_FILE}" ]; then
    print_warning "New service file already exists at ${NEW_DEST_FILE}"
    read -p "Overwrite existing service file? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Keeping existing service file"
    fi
fi

# Confirm migration
echo ""
read -p "Proceed with migration? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_info "Migration cancelled"
    exit 0
fi

echo ""
print_info "Starting migration..."

# Step 1: Stop old service if running
if [ "$OLD_RUNNING" = true ]; then
    print_status "Stopping ${OLD_SERVICE}..."
    systemctl stop ${OLD_SERVICE}.service
    sleep 2
fi

# Step 2: Disable old service
if [ "$OLD_ENABLED" = true ]; then
    print_status "Disabling ${OLD_SERVICE} auto-start..."
    systemctl disable ${OLD_SERVICE}.service 2>/dev/null || true
fi

# Step 3: Install new service
print_status "Installing ${NEW_SERVICE}..."
cp "${NEW_SERVICE_FILE}" "${NEW_DEST_FILE}"

# Step 4: Reload systemd
print_status "Reloading systemd daemon..."
systemctl daemon-reload

# Step 5: Enable new service if old was enabled
if [ "$OLD_ENABLED" = true ]; then
    print_status "Enabling ${NEW_SERVICE} for auto-start..."
    systemctl enable ${NEW_SERVICE}.service
fi

# Step 6: Start new service if old was running
if [ "$OLD_RUNNING" = true ]; then
    print_status "Starting ${NEW_SERVICE}..."
    systemctl start ${NEW_SERVICE}.service
    
    # Wait and check status
    sleep 3
    if systemctl is-active --quiet ${NEW_SERVICE}.service; then
        print_status "${NEW_SERVICE} started successfully"
    else
        print_error "${NEW_SERVICE} failed to start"
        echo ""
        echo "Check logs with:"
        echo "  journalctl -u ${NEW_SERVICE} -n 50"
        echo ""
        echo "You may need to:"
        echo "1. Ensure eth_data package is installed"
        echo "2. Check database connectivity"
        echo "3. Verify Python environment"
        exit 1
    fi
fi

# Step 7: Remove old service file
print_status "Removing old service file..."
rm -f "${OLD_SERVICE_FILE}"

# Step 8: Final cleanup
print_status "Cleaning up..."
systemctl daemon-reload

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}   Migration Complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Show status
echo "Service Status:"
if systemctl is-active --quiet ${NEW_SERVICE}.service; then
    echo -e "  ${GREEN}●${NC} ${NEW_SERVICE} is running"
else
    echo -e "  ${RED}●${NC} ${NEW_SERVICE} is not running"
fi

if systemctl is-enabled --quiet ${NEW_SERVICE}.service 2>/dev/null; then
    echo -e "  ${GREEN}●${NC} ${NEW_SERVICE} is enabled (will start on boot)"
else
    echo -e "  ${YELLOW}●${NC} ${NEW_SERVICE} is disabled"
fi

echo ""
echo "Useful commands:"
echo "  View status:  systemctl status ${NEW_SERVICE}"
echo "  View logs:    journalctl -u ${NEW_SERVICE} -f"
echo "  Stop service: sudo systemctl stop ${NEW_SERVICE}"
echo "  Start service: sudo systemctl start ${NEW_SERVICE}"
echo ""

# Show recent logs
echo "Recent logs from ${NEW_SERVICE}:"
echo "--------------------------------"
journalctl -u ${NEW_SERVICE} -n 10 --no-pager