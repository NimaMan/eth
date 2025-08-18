# ETH Data Scripts

This directory contains scripts for running and managing ETH Data components.

## Directory Structure

```
scripts/
├── services/                  # Service management (see services/README.md)
│   ├── systemd/              # Service definition files
│   ├── install_service.sh    # Service installer
│   ├── manage_service.sh     # Quick management
│   ├── migrate_service.sh    # Migration tool
│   └── service_manager.sh    # Master service manager
├── process_blocks_live.py    # Live block processor script
├── run_live_processor.sh     # Development runner
└── provide_tx_service/       # Transaction validation service
```

## Quick Start

### For Service Management

All service-related operations are in the `services/` directory:

```bash
cd services/

# Install and start the Live Block Processor service
sudo ./install_service.sh install

# Or use the master manager for all services
./service_manager.sh
```

See [services/README.md](services/README.md) for detailed service documentation.

### For Development

Run the processor directly without systemd:
```bash
./run_live_processor.sh
```

## Core Scripts

### process_blocks_live.py
Main Python script that runs the Live Block Processor. Monitors new blocks and processes transactions in real-time.

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