#!/bin/bash

# Quick management script for the Ethereum Live Block Processor service
# Usage: ./manage_service.sh [start|stop|restart|status|logs|follow]

SERVICE_NAME="eth-live-block-processor"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[i]${NC} $1"
}

check_service_exists() {
    if ! systemctl list-units --all | grep -q "${SERVICE_NAME}.service"; then
        print_error "Service ${SERVICE_NAME} is not installed"
        print_info "Run: sudo ./install_service.sh install"
        exit 1
    fi
}

case "$1" in
    start)
        check_service_exists
        sudo systemctl start ${SERVICE_NAME}.service
        print_status "Service started"
        sleep 1
        systemctl is-active --quiet ${SERVICE_NAME}.service && \
            print_status "Service is running" || \
            print_error "Service failed to start"
        ;;
    
    stop)
        check_service_exists
        sudo systemctl stop ${SERVICE_NAME}.service
        print_status "Service stopped"
        ;;
    
    restart)
        check_service_exists
        sudo systemctl restart ${SERVICE_NAME}.service
        print_status "Service restarted"
        sleep 1
        systemctl is-active --quiet ${SERVICE_NAME}.service && \
            print_status "Service is running" || \
            print_error "Service failed to restart"
        ;;
    
    status)
        check_service_exists
        systemctl status ${SERVICE_NAME}.service --no-pager
        ;;
    
    logs)
        check_service_exists
        journalctl -u ${SERVICE_NAME} -n 100 --no-pager
        ;;
    
    follow)
        check_service_exists
        print_info "Following logs (Ctrl+C to exit)..."
        journalctl -u ${SERVICE_NAME} -f
        ;;
    
    errors)
        check_service_exists
        print_info "Showing errors from the last hour..."
        journalctl -u ${SERVICE_NAME} --since "1 hour ago" -p err --no-pager
        ;;
    
    *)
        echo "Usage: $0 {start|stop|restart|status|logs|follow|errors}"
        echo ""
        echo "Commands:"
        echo "  start   - Start the service"
        echo "  stop    - Stop the service"
        echo "  restart - Restart the service"
        echo "  status  - Show service status"
        echo "  logs    - Show recent logs (last 100 lines)"
        echo "  follow  - Follow logs in real-time"
        echo "  errors  - Show errors from the last hour"
        exit 1
        ;;
esac