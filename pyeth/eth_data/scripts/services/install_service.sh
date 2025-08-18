#!/bin/bash

# Install and manage the Ethereum Live Block Processor service
# This script handles installation, updates, and management of the systemd service

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Service details
SERVICE_NAME="eth-live-block-processor"
SERVICE_FILE="systemd/${SERVICE_NAME}.service"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SOURCE_FILE="${SCRIPT_DIR}/${SERVICE_FILE}"
DEST_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

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

# Check if running with sudo
check_sudo() {
    if [ "$EUID" -ne 0 ]; then 
        print_error "This script must be run with sudo"
        exit 1
    fi
}

# Stop the old service if it exists
stop_old_service() {
    OLD_SERVICE="eth-block-processor"
    if systemctl list-units --all | grep -q "${OLD_SERVICE}.service"; then
        print_warning "Found old service ${OLD_SERVICE}, stopping and disabling..."
        systemctl stop ${OLD_SERVICE}.service 2>/dev/null || true
        systemctl disable ${OLD_SERVICE}.service 2>/dev/null || true
        rm -f /etc/systemd/system/${OLD_SERVICE}.service
        print_status "Old service removed"
    fi
}

# Install the service
install_service() {
    print_status "Installing ${SERVICE_NAME} service..."
    
    # Check if source file exists
    if [ ! -f "${SOURCE_FILE}" ]; then
        print_error "Service file not found: ${SOURCE_FILE}"
        exit 1
    fi
    
    # Copy service file
    cp "${SOURCE_FILE}" "${DEST_FILE}"
    print_status "Service file copied to ${DEST_FILE}"
    
    # Reload systemd daemon
    systemctl daemon-reload
    print_status "Systemd daemon reloaded"
    
    # Enable service
    systemctl enable ${SERVICE_NAME}.service
    print_status "Service enabled for automatic startup"
}

# Start the service
start_service() {
    print_status "Starting ${SERVICE_NAME} service..."
    systemctl start ${SERVICE_NAME}.service
    
    # Wait a moment for service to start
    sleep 2
    
    # Check status
    if systemctl is-active --quiet ${SERVICE_NAME}.service; then
        print_status "Service started successfully"
    else
        print_error "Service failed to start"
        echo "Check logs with: journalctl -u ${SERVICE_NAME} -n 50"
        exit 1
    fi
}

# Update an existing service
update_service() {
    print_status "Updating ${SERVICE_NAME} service..."
    
    # Stop current service
    if systemctl is-active --quiet ${SERVICE_NAME}.service; then
        systemctl stop ${SERVICE_NAME}.service
        print_status "Service stopped"
    fi
    
    # Install new version
    install_service
    
    # Start service
    start_service
}

# Show service status
show_status() {
    echo ""
    echo "=== Service Status ==="
    systemctl status ${SERVICE_NAME}.service --no-pager
    echo ""
    echo "=== Recent Logs ==="
    journalctl -u ${SERVICE_NAME} -n 20 --no-pager
}

# Main menu
show_menu() {
    echo ""
    echo "Ethereum Live Block Processor Service Manager"
    echo "=============================================="
    echo "1) Install service (first time)"
    echo "2) Update service (already installed)"
    echo "3) Start service"
    echo "4) Stop service"
    echo "5) Restart service"
    echo "6) Show status and logs"
    echo "7) Uninstall service"
    echo "8) Exit"
    echo ""
}

# Uninstall service
uninstall_service() {
    print_warning "Uninstalling ${SERVICE_NAME} service..."
    
    # Stop service
    systemctl stop ${SERVICE_NAME}.service 2>/dev/null || true
    
    # Disable service
    systemctl disable ${SERVICE_NAME}.service 2>/dev/null || true
    
    # Remove service file
    rm -f ${DEST_FILE}
    
    # Reload daemon
    systemctl daemon-reload
    
    print_status "Service uninstalled"
}

# Handle command line arguments
if [ $# -gt 0 ]; then
    check_sudo
    case "$1" in
        install)
            stop_old_service
            install_service
            start_service
            show_status
            ;;
        update)
            update_service
            show_status
            ;;
        start)
            systemctl start ${SERVICE_NAME}.service
            print_status "Service started"
            ;;
        stop)
            systemctl stop ${SERVICE_NAME}.service
            print_status "Service stopped"
            ;;
        restart)
            systemctl restart ${SERVICE_NAME}.service
            print_status "Service restarted"
            ;;
        status)
            show_status
            ;;
        uninstall)
            uninstall_service
            ;;
        *)
            echo "Usage: $0 {install|update|start|stop|restart|status|uninstall}"
            exit 1
            ;;
    esac
else
    # Interactive mode
    check_sudo
    
    while true; do
        show_menu
        read -p "Select option: " choice
        
        case $choice in
            1)
                stop_old_service
                install_service
                start_service
                show_status
                ;;
            2)
                update_service
                show_status
                ;;
            3)
                systemctl start ${SERVICE_NAME}.service
                print_status "Service started"
                ;;
            4)
                systemctl stop ${SERVICE_NAME}.service
                print_status "Service stopped"
                ;;
            5)
                systemctl restart ${SERVICE_NAME}.service
                print_status "Service restarted"
                ;;
            6)
                show_status
                ;;
            7)
                uninstall_service
                ;;
            8)
                print_status "Exiting..."
                exit 0
                ;;
            *)
                print_error "Invalid option"
                ;;
        esac
        
        echo ""
        read -p "Press Enter to continue..."
    done
fi