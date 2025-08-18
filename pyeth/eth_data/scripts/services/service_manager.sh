#!/bin/bash

# Master Service Manager for ETH Data Services
# Manages all eth-* services from a single interface

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Service configurations
declare -A SERVICES
SERVICES["eth-live-block-processor"]="Live Block Processor"
# Add more services here as they're created
# SERVICES["eth-mempool-processor"]="Mempool Processor"
# SERVICES["eth-alert-manager"]="Alert Manager"

# Script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Functions for colored output
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

print_service() {
    echo -e "${CYAN}[S]${NC} $1"
}

# Check if running with sudo for certain operations
check_sudo() {
    if [ "$EUID" -ne 0 ]; then 
        return 1
    fi
    return 0
}

# Get service status with color
get_service_status() {
    local service=$1
    if systemctl is-active --quiet "$service"; then
        echo -e "${GREEN}● Running${NC}"
    elif systemctl is-enabled --quiet "$service" 2>/dev/null; then
        echo -e "${YELLOW}● Stopped${NC}"
    else
        echo -e "${RED}● Not Installed${NC}"
    fi
}

# Show status of all services
show_all_status() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}            ETH Data Services Status${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo ""
    
    for service in "${!SERVICES[@]}"; do
        local name="${SERVICES[$service]}"
        local status=$(get_service_status "$service")
        printf "  %-30s %s\n" "$name:" "$status"
        
        # Show additional info if running
        if systemctl is-active --quiet "$service"; then
            local pid=$(systemctl show -p MainPID --value "$service")
            local memory=$(systemctl show -p MemoryCurrent --value "$service")
            if [ "$memory" != "[not set]" ] && [ "$memory" != "18446744073709551615" ]; then
                memory_mb=$((memory / 1024 / 1024))
                echo -e "    ${CYAN}PID:${NC} $pid  ${CYAN}Memory:${NC} ${memory_mb}MB"
            fi
        fi
    done
    echo ""
}

# Start all services
start_all() {
    print_info "Starting all services..."
    local failed=0
    
    for service in "${!SERVICES[@]}"; do
        if systemctl list-units --all | grep -q "$service.service"; then
            if ! systemctl is-active --quiet "$service"; then
                print_info "Starting ${SERVICES[$service]}..."
                if sudo systemctl start "$service" 2>/dev/null; then
                    print_status "${SERVICES[$service]} started"
                else
                    print_error "Failed to start ${SERVICES[$service]}"
                    failed=$((failed + 1))
                fi
            else
                print_info "${SERVICES[$service]} already running"
            fi
        else
            print_warning "${SERVICES[$service]} not installed"
        fi
    done
    
    if [ $failed -eq 0 ]; then
        print_status "All services started successfully"
    else
        print_warning "$failed service(s) failed to start"
    fi
}

# Stop all services
stop_all() {
    print_info "Stopping all services..."
    
    for service in "${!SERVICES[@]}"; do
        if systemctl is-active --quiet "$service"; then
            print_info "Stopping ${SERVICES[$service]}..."
            if sudo systemctl stop "$service" 2>/dev/null; then
                print_status "${SERVICES[$service]} stopped"
            else
                print_error "Failed to stop ${SERVICES[$service]}"
            fi
        fi
    done
    
    print_status "All services stopped"
}

# Restart all services
restart_all() {
    print_info "Restarting all services..."
    stop_all
    echo ""
    start_all
}

# Show logs for a service
show_logs() {
    local service=$1
    local lines=${2:-50}
    
    if [ -z "$service" ]; then
        print_error "No service specified"
        return 1
    fi
    
    if systemctl list-units --all | grep -q "$service.service"; then
        journalctl -u "$service" -n "$lines" --no-pager
    else
        print_error "Service $service not found"
    fi
}

# Follow logs for a service
follow_logs() {
    local service=$1
    
    if [ -z "$service" ]; then
        # Show combined logs for all services
        print_info "Following logs for all ETH services (Ctrl+C to exit)..."
        local services_pattern=$(echo "${!SERVICES[@]}" | tr ' ' '|')
        journalctl -u "eth-*" -f
    else
        if systemctl list-units --all | grep -q "$service.service"; then
            print_info "Following logs for $service (Ctrl+C to exit)..."
            journalctl -u "$service" -f
        else
            print_error "Service $service not found"
        fi
    fi
}

# Interactive service selector
select_service() {
    echo ""
    echo "Select a service:"
    echo ""
    
    local i=1
    local service_array=()
    
    for service in "${!SERVICES[@]}"; do
        service_array+=("$service")
        local status=$(get_service_status "$service")
        printf "  %d) %-30s %s\n" "$i" "${SERVICES[$service]}" "$status"
        i=$((i + 1))
    done
    
    echo ""
    read -p "Enter number (1-${#SERVICES[@]}): " choice
    
    if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le "${#SERVICES[@]}" ]; then
        echo "${service_array[$((choice-1))]}"
    else
        echo ""
    fi
}

# Show service menu
show_service_menu() {
    local service=$1
    local name="${SERVICES[$service]}"
    
    while true; do
        echo ""
        echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
        echo -e "${BLUE}     $name${NC}"
        echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
        echo ""
        echo "  Status: $(get_service_status "$service")"
        echo ""
        echo "  1) Start service"
        echo "  2) Stop service"
        echo "  3) Restart service"
        echo "  4) Show status"
        echo "  5) Show logs (last 50)"
        echo "  6) Follow logs"
        echo "  7) Show errors"
        echo "  8) Back to main menu"
        echo ""
        read -p "Select option: " choice
        
        case $choice in
            1)
                sudo systemctl start "$service"
                print_status "Service started"
                ;;
            2)
                sudo systemctl stop "$service"
                print_status "Service stopped"
                ;;
            3)
                sudo systemctl restart "$service"
                print_status "Service restarted"
                ;;
            4)
                systemctl status "$service" --no-pager
                ;;
            5)
                show_logs "$service" 50
                ;;
            6)
                follow_logs "$service"
                ;;
            7)
                journalctl -u "$service" --since "1 hour ago" -p err --no-pager
                ;;
            8)
                return
                ;;
            *)
                print_error "Invalid option"
                ;;
        esac
        
        if [ "$choice" != "8" ]; then
            echo ""
            read -p "Press Enter to continue..."
        fi
    done
}

# Main menu
show_main_menu() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}        ETH Data Service Manager${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo ""
    echo "  1) Show all services status"
    echo "  2) Start all services"
    echo "  3) Stop all services"
    echo "  4) Restart all services"
    echo "  5) Manage individual service"
    echo "  6) Follow all logs"
    echo "  7) Install/Update services"
    echo "  8) Migrate from old services"
    echo "  9) Exit"
    echo ""
}

# Handle command line arguments
if [ $# -gt 0 ]; then
    case "$1" in
        status)
            show_all_status
            ;;
        start)
            if [ -z "$2" ]; then
                start_all
            else
                sudo systemctl start "$2"
                print_status "$2 started"
            fi
            ;;
        stop)
            if [ -z "$2" ]; then
                stop_all
            else
                sudo systemctl stop "$2"
                print_status "$2 stopped"
            fi
            ;;
        restart)
            if [ -z "$2" ]; then
                restart_all
            else
                sudo systemctl restart "$2"
                print_status "$2 restarted"
            fi
            ;;
        logs)
            if [ -z "$2" ]; then
                follow_logs
            else
                show_logs "$2" "${3:-50}"
            fi
            ;;
        follow)
            follow_logs "$2"
            ;;
        *)
            echo "Usage: $0 {status|start|stop|restart|logs|follow} [service-name]"
            echo ""
            echo "Examples:"
            echo "  $0 status                    # Show all services status"
            echo "  $0 start                     # Start all services"
            echo "  $0 stop eth-live-block-processor  # Stop specific service"
            echo "  $0 logs                      # Show all services logs"
            echo "  $0 follow eth-live-block-processor # Follow specific service logs"
            exit 1
            ;;
    esac
else
    # Interactive mode
    while true; do
        show_main_menu
        read -p "Select option: " choice
        
        case $choice in
            1)
                show_all_status
                ;;
            2)
                if check_sudo; then
                    start_all
                else
                    print_warning "This operation requires sudo"
                    sudo "$0" start
                fi
                ;;
            3)
                if check_sudo; then
                    stop_all
                else
                    print_warning "This operation requires sudo"
                    sudo "$0" stop
                fi
                ;;
            4)
                if check_sudo; then
                    restart_all
                else
                    print_warning "This operation requires sudo"
                    sudo "$0" restart
                fi
                ;;
            5)
                selected=$(select_service)
                if [ -n "$selected" ]; then
                    show_service_menu "$selected"
                fi
                ;;
            6)
                follow_logs
                ;;
            7)
                if check_sudo; then
                    "${SCRIPT_DIR}/install_service.sh"
                else
                    print_warning "This operation requires sudo"
                    sudo "${SCRIPT_DIR}/install_service.sh"
                fi
                ;;
            8)
                if check_sudo; then
                    "${SCRIPT_DIR}/migrate_service.sh"
                else
                    print_warning "This operation requires sudo"
                    sudo "${SCRIPT_DIR}/migrate_service.sh"
                fi
                ;;
            9)
                print_status "Exiting..."
                exit 0
                ;;
            *)
                print_error "Invalid option"
                ;;
        esac
        
        if [ "$choice" != "9" ]; then
            echo ""
            read -p "Press Enter to continue..."
        fi
    done
fi