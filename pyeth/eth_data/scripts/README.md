# Ethereum Live Block Processor Scripts

This directory contains scripts and systemd service files for running the Ethereum Live Block Processor as a system service.

## Overview

The Live Block Processor continuously monitors the Ethereum blockchain, processing new blocks in real-time and storing transaction data in PostgreSQL. It uses the latest `eth_data` package (formerly `eth_block_processor`).

## Files

### Service Files
- `systemd/eth-live-block-processor.service` - Systemd service definition file

### Management Scripts
- `install_service.sh` - Install/update/manage the systemd service (requires sudo)
- `manage_service.sh` - Quick service management (start/stop/restart/logs)
- `run_live_processor.sh` - Run the processor directly for testing/development

### Core Script
- `process_blocks_live.py` - Main Python script that runs the Live Block Processor

## Installation

### 1. Install the Service (First Time)

```bash
sudo ./install_service.sh install
```

This will:
- Stop and remove any old `eth-block-processor` service
- Install the new `eth-live-block-processor` service
- Enable it for automatic startup
- Start the service

### 2. Update Existing Service

If the service is already installed and you want to update it with the latest code:

```bash
sudo ./install_service.sh update
```

## Usage

### Quick Management

Use the `manage_service.sh` script for quick operations:

```bash
# Start the service
./manage_service.sh start

# Stop the service
./manage_service.sh stop

# Restart the service
./manage_service.sh restart

# Check status
./manage_service.sh status

# View recent logs
./manage_service.sh logs

# Follow logs in real-time
./manage_service.sh follow

# Show errors from the last hour
./manage_service.sh errors
```

### Development/Testing

To run the processor directly without systemd (useful for debugging):

```bash
./run_live_processor.sh
```

This will:
- Set up the required environment variables
- Check dependencies (PostgreSQL, RabbitMQ)
- Run the processor in the foreground
- Show output directly in the terminal

### Systemctl Commands

You can also use standard systemctl commands:

```bash
# Start service
sudo systemctl start eth-live-block-processor

# Stop service
sudo systemctl stop eth-live-block-processor

# Restart service
sudo systemctl restart eth-live-block-processor

# Check status
systemctl status eth-live-block-processor

# View logs
journalctl -u eth-live-block-processor -f

# View logs from the last hour
journalctl -u eth-live-block-processor --since "1 hour ago"
```

## Configuration

The service is configured via environment variables in the service file:

- `PYTHONPATH`: Set to `/home/nima/code/crypto/py/eth_data`
- `DATABASE_URL`: PostgreSQL connection string
- `RABBITMQ_URL`: RabbitMQ connection string (for message queue)
- `LOG_LEVEL`: Logging level (default: INFO)
- `PYTHONUNBUFFERED`: Set to 1 for immediate log output

## Requirements

- Python environment: `qw` conda environment with required packages
- PostgreSQL: Database server running on localhost:5432
- RabbitMQ: Message queue server (optional, for alerts)
- eth_data package: Must be installed in the qw environment

## Troubleshooting

### Service Won't Start

Check the logs for errors:
```bash
journalctl -u eth-live-block-processor -n 50
```

Common issues:
- PostgreSQL not running
- Missing Python packages in qw environment
- Database connection issues

### Old Service Still Running

The install script automatically stops and removes the old `eth-block-processor` service, but if needed:

```bash
sudo systemctl stop eth-block-processor
sudo systemctl disable eth-block-processor
sudo rm /etc/systemd/system/eth-block-processor.service
sudo systemctl daemon-reload
```

### Permission Issues

Make sure the service runs as the correct user (nima) and has access to:
- The code directory
- The PostgreSQL database
- The conda environment

## Migration from eth-block-processor

This service replaces the old `eth-block-processor` service. Key changes:
- Package renamed from `eth_block_processor` to `eth_data`
- Service renamed to `eth-live-block-processor`
- Updated paths and imports
- Improved service management scripts

The install script automatically handles the migration by stopping and removing the old service.