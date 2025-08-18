#!/bin/bash

# Install Transaction Validation Service as systemd service
# This script sets up the Python validation service to run automatically

set -e

echo "🚀 Installing Transaction Validation Service as systemd service..."

# Check if running as root (needed for systemd)
if [ "$EUID" -ne 0 ]; then
    echo "❌ This script must be run as root (use sudo)"
    echo "   Usage: sudo ./install_system_service.sh"
    exit 1
fi

# Get the actual user (not root when using sudo)
ACTUAL_USER=${SUDO_USER:-$USER}
ACTUAL_HOME=$(eval echo ~$ACTUAL_USER)

echo "📋 Installing for user: $ACTUAL_USER"
echo "📁 Home directory: $ACTUAL_HOME"

# Verify service file exists
SERVICE_FILE="$PWD/../systemd/tx-validation.service"
if [ ! -f "$SERVICE_FILE" ]; then
    echo "❌ Service file not found: $SERVICE_FILE"
    echo "   Make sure you're running this from the scripts directory"
    exit 1
fi

# Verify Python validation service exists
VALIDATION_SERVICE="$PWD/../validation_service.py"
if [ ! -f "$VALIDATION_SERVICE" ]; then
    echo "❌ Validation service not found: $VALIDATION_SERVICE"
    exit 1
fi

# Copy service file to systemd directory
echo "📄 Installing service file..."
cp "$SERVICE_FILE" /etc/systemd/system/
chown root:root /etc/systemd/system/tx-validation.service
chmod 644 /etc/systemd/system/tx-validation.service

# Reload systemd
echo "🔄 Reloading systemd..."
systemctl daemon-reload

# Enable service
echo "✅ Enabling service..."
systemctl enable tx-validation.service

# Check service status
echo ""
echo "📊 Service Status:"
systemctl status tx-validation.service --no-pager --lines=0 || true

echo ""
echo "🎯 Installation Complete!"
echo ""
echo "Service Management Commands:"
echo "  Start:   sudo systemctl start tx-validation"
echo "  Stop:    sudo systemctl stop tx-validation"
echo "  Restart: sudo systemctl restart tx-validation"
echo "  Status:  sudo systemctl status tx-validation"
echo "  Logs:    sudo journalctl -u tx-validation -f"
echo ""
echo "Service will be available at: http://127.0.0.1:18000"
echo "Health check: curl http://127.0.0.1:18000/health"
echo ""
echo "To start the service now, run:"
echo "  sudo systemctl start tx-validation"