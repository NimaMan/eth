# ETH Data Services

This directory contains systemd service definitions and management scripts for various eth_data components.

## Directory Structure

```
services/
├── systemd/                          # Service definition files
│   ├── eth-live-block-processor.service
│   └── eth-mempool-processor.service (future)
├── install_service.sh                # Universal service installer
├── manage_service.sh                 # Quick service management
├── migrate_service.sh                # Migration from old services
└── README.md                         # This file
```

## Available Services

### 1. Live Block Processor (`eth-live-block-processor`)

Continuously processes new Ethereum blocks in real-time.

**Features:**
- Monitors new blocks as they're mined
- Processes and stores transaction data
- Publishes alerts via RabbitMQ
- Maintains database integrity

**Status:** ✅ Active

### 2. Mempool Processor (Coming Soon)

Will monitor and process mempool transactions.

**Features:**
- Real-time mempool monitoring
- Transaction analysis before confirmation
- MEV detection
- Gas price tracking

**Status:** 🚧 Planned

## Installation

### Quick Install

For a single service:
```bash
cd /home/nima/code/crypto/py/eth_data/scripts/services
sudo ./install_service.sh install
```

### Migration from Old Services

If you have old services running (e.g., `eth-block-processor`):
```bash
sudo ./migrate_service.sh
```

## Management

### Using manage_service.sh

Quick commands for any service:
```bash
# Start service
./manage_service.sh start

# Stop service
./manage_service.sh stop

# Restart service
./manage_service.sh restart

# View status
./manage_service.sh status

# Follow logs
./manage_service.sh follow

# Show errors
./manage_service.sh errors
```

### Using systemctl directly

```bash
# Start
sudo systemctl start eth-live-block-processor

# Stop
sudo systemctl stop eth-live-block-processor

# Status
systemctl status eth-live-block-processor

# Logs
journalctl -u eth-live-block-processor -f
```

## Service Configuration

All services use these common environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `PYTHONPATH` | Python module path | `/home/nima/code/crypto/py/eth_data` |
| `DATABASE_URL` | PostgreSQL connection | `postgresql://postgres:postgres@localhost:5432/eth_db` |
| `RABBITMQ_URL` | RabbitMQ connection | `amqp://guest:guest@127.0.0.1/` |
| `LOG_LEVEL` | Logging verbosity | `INFO` |
| `PYTHONUNBUFFERED` | Immediate log output | `1` |

## Adding New Services

To add a new service:

1. Create service file in `systemd/` directory
2. Use this template:

```ini
[Unit]
Description=Your Service Description
After=network.target postgresql.service
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=5
User=nima
Group=nima

# Environment
Environment="PYTHONPATH=/home/nima/code/crypto/py/eth_data"
Environment="DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db"
Environment="LOG_LEVEL=INFO"

# Execution
ExecStart=/home/nima/miniconda3/envs/qw/bin/python /path/to/your/script.py
WorkingDirectory=/home/nima/code/crypto/py/eth_data

# Logging
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

3. Install with: `sudo ./install_service.sh install`

## Troubleshooting

### Service Won't Start

1. Check logs:
```bash
journalctl -u eth-live-block-processor -n 100
```

2. Verify dependencies:
- PostgreSQL is running
- RabbitMQ is accessible (if needed)
- Python environment has required packages

3. Test manually:
```bash
cd /home/nima/code/crypto/py/eth_data/scripts
./run_live_processor.sh
```

### Permission Issues

Ensure the service user (nima) has access to:
- Code directory: `/home/nima/code/crypto/py/eth_data`
- Python environment: `/home/nima/miniconda3/envs/qw`
- Database: Check PostgreSQL user permissions

### High Memory Usage

Services have a 4GB memory limit by default. Adjust in service file:
```ini
MemoryLimit=8G  # Increase to 8GB
```

## Monitoring

### Check All ETH Services

```bash
systemctl list-units 'eth-*'
```

### Resource Usage

```bash
systemctl status eth-live-block-processor | grep Memory
```

### Performance Metrics

View service performance:
```bash
systemd-cgtop
```

## Best Practices

1. **Always use migration script** when updating from old services
2. **Check logs** after any service restart
3. **Monitor memory usage** for long-running services
4. **Use manage_service.sh** for quick operations
5. **Keep services updated** with latest code changes

## Support

For issues or questions:
1. Check logs first: `journalctl -u service-name -n 100`
2. Run manually to debug: `./run_live_processor.sh`
3. Verify database connectivity
4. Ensure Python packages are installed in qw environment