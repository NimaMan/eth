#!/bin/bash

# Transaction Validation Service Management Script
# Convenient commands for managing the systemd service

SERVICE_NAME="tx-validation"
SERVICE_URL="http://127.0.0.1:18000"

show_usage() {
    echo "Transaction Validation Service Management"
    echo "========================================"
    echo ""
    echo "Usage: $0 {start|stop|restart|status|logs|health|install|uninstall}"
    echo ""
    echo "Commands:"
    echo "  start     - Start the validation service"
    echo "  stop      - Stop the validation service"
    echo "  restart   - Restart the validation service"
    echo "  status    - Show service status"
    echo "  logs      - Show service logs (real-time)"
    echo "  health    - Check service health endpoint"
    echo "  install   - Install as systemd service"
    echo "  uninstall - Remove systemd service"
    echo ""
    echo "Service URL: $SERVICE_URL"
}

check_service_exists() {
    if ! systemctl list-unit-files | grep -q "^$SERVICE_NAME.service"; then
        echo "❌ Service not installed. Run: $0 install"
        exit 1
    fi
}

case "$1" in
    start)
        check_service_exists
        echo "🚀 Starting $SERVICE_NAME service..."
        sudo systemctl start $SERVICE_NAME
        sleep 2
        sudo systemctl status $SERVICE_NAME --no-pager --lines=3
        echo ""
        echo "✅ Service started. Health check: $0 health"
        ;;
    
    stop)
        check_service_exists
        echo "⏹️  Stopping $SERVICE_NAME service..."
        sudo systemctl stop $SERVICE_NAME
        sudo systemctl status $SERVICE_NAME --no-pager --lines=3
        ;;
    
    restart)
        check_service_exists
        echo "🔄 Restarting $SERVICE_NAME service..."
        sudo systemctl restart $SERVICE_NAME
        sleep 2
        sudo systemctl status $SERVICE_NAME --no-pager --lines=3
        echo ""
        echo "✅ Service restarted. Health check: $0 health"
        ;;
    
    status)
        check_service_exists
        echo "📊 Service Status:"
        sudo systemctl status $SERVICE_NAME --no-pager
        ;;
    
    logs)
        check_service_exists
        echo "📋 Service Logs (press Ctrl+C to exit):"
        sudo journalctl -u $SERVICE_NAME -f
        ;;
    
    health)
        echo "🏥 Checking service health..."
        if curl -s "$SERVICE_URL/health" > /dev/null 2>&1; then
            echo "✅ Service is healthy"
            curl -s "$SERVICE_URL/health" | python3 -m json.tool
        else
            echo "❌ Service is not responding"
            echo "   URL: $SERVICE_URL/health"
            echo "   Check if service is running: $0 status"
        fi
        ;;
    
    install)
        echo "📦 Installing $SERVICE_NAME as systemd service..."
        if [ ! -f "install_system_service.sh" ]; then
            echo "❌ install_system_service.sh not found in current directory"
            exit 1
        fi
        sudo ./install_system_service.sh
        ;;
    
    uninstall)
        echo "🗑️  Uninstalling $SERVICE_NAME service..."
        if systemctl list-unit-files | grep -q "^$SERVICE_NAME.service"; then
            sudo systemctl stop $SERVICE_NAME 2>/dev/null || true
            sudo systemctl disable $SERVICE_NAME
            sudo rm -f /etc/systemd/system/$SERVICE_NAME.service
            sudo systemctl daemon-reload
            echo "✅ Service uninstalled"
        else
            echo "⚠️  Service was not installed"
        fi
        ;;
    
    *)
        show_usage
        exit 1
        ;;
esac