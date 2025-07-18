# Transaction Validation System Service

This guide sets up the Python transaction validation service as a systemd service that runs automatically in the background.

## 🎯 **Overview**

The validation service provides transaction processing and state change analysis via HTTP API on port 18000. It's designed to work seamlessly with the Rust components for high-performance blockchain analytics.

## ✅ **Features**

- **Always Available** - Service runs on port 18000 automatically
- **Auto-Start** - Starts on system boot
- **Auto-Restart** - Restarts if it crashes  
- **Resource Limits** - 2GB RAM, 200% CPU (2 cores)
- **Security Hardening** - Runs as non-root user with restricted permissions
- **Proper Logging** - Centralized logging with journald
- **Health Monitoring** - `/health` endpoint for status checks

## 🚀 **Quick Setup**

```bash
cd /home/nima/code/crypto/py/eth_block_processor/scripts/provide_tx_service

# Install as system service (one-time setup)
sudo ./scripts/install_system_service.sh

# Start the service
./scripts/service_management.sh start

# Check it's working
./scripts/service_management.sh health
```

## 📋 **Configuration Details**

- **Port**: `18000` (non-conflicting with common services)
- **URL**: `http://127.0.0.1:18000`
- **User**: `nima` (your user account)
- **Working Directory**: `/home/nima/code/crypto/py/eth_block_processor/scripts/provide_tx_service`
- **Environment**: Conda `qw` environment activated
- **Memory Limit**: 2GB
- **CPU Limit**: 200% (2 cores)
- **Restart Policy**: Always restart on failure

## 🛠️ **Management Commands**

```bash
# Service management via wrapper script
./scripts/service_management.sh start     # Start service
./scripts/service_management.sh stop      # Stop service  
./scripts/service_management.sh restart   # Restart service
./scripts/service_management.sh status    # Show status
./scripts/service_management.sh logs      # View logs (real-time)
./scripts/service_management.sh health    # Test health endpoint
./scripts/service_management.sh uninstall # Remove system service

# Alternative: Direct systemctl commands
sudo systemctl start tx-validation
sudo systemctl stop tx-validation
sudo systemctl status tx-validation
sudo journalctl -u tx-validation -f
```

## 📊 **API Endpoints**

Once running, the service provides:

- **Health Check**: `GET http://127.0.0.1:18000/health`
- **Single Transaction**: `POST http://127.0.0.1:18000/validate/transaction/{tx_hash}`
- **Batch Processing**: `POST http://127.0.0.1:18000/validate/batch`
- **API Documentation**: `GET http://127.0.0.1:18000/docs`

## 🔍 **Verification**

Test that everything works:

```bash
# 1. Check service status
./scripts/service_management.sh status

# 2. Test health endpoint
curl http://127.0.0.1:18000/health

# 3. Test with reference transaction
./scripts/test_reference_transaction.sh

# 4. View real-time logs
./scripts/service_management.sh logs
```

## 🚨 **Troubleshooting**

### Service Won't Start
```bash
# Check detailed status
sudo systemctl status tx-validation -l

# Check logs for errors
sudo journalctl -u tx-validation --no-pager

# Restart the service
sudo systemctl restart tx-validation

# Verify conda environment
source /home/nima/miniconda3/etc/profile.d/conda.sh
conda activate qw
python -c "import web3; print('Dependencies OK')"
```

### Port Already in Use
```bash
# Check what's using the port
sudo lsof -i :18000

# Kill the process if needed
sudo kill -9 <PID>
```

### Permission Issues
```bash
# Check file permissions
ls -la /home/nima/code/crypto/py/eth_block_processor/scripts/provide_tx_service/

# Fix if needed
chmod +x scripts/*.sh
```

## 🔧 **Files Overview**

### Service Files
- `systemd/tx-validation.service` - Main systemd service configuration
- `systemd/tx-validation-simple.service` - Simplified debug configuration

### Shell Scripts
- `scripts/install_system_service.sh` - One-time installation script
- `scripts/service_management.sh` - Convenient management commands
- `scripts/service_wrapper.sh` - Wrapper that activates conda environment
- `start_service.sh` - Manual startup for development
- `scripts/test_reference_transaction.sh` - Test with known transaction

### Python Files
- `validation_service.py` - Main FastAPI service implementation
- `setup_validation_service.py` - Setup script for Python paths

### Dependencies
- `requirements_validation.txt` - Service-specific Python packages

## 🔄 **Updates**

When you update the validation service code:

```bash
# Simply restart the service
./scripts/service_management.sh restart
```

The service will pick up the latest code automatically.

## 📈 **Monitoring**

The service logs everything to journald. Key metrics to monitor:

- **Service Status**: `systemctl is-active tx-validation`
- **Memory Usage**: Shown in `systemctl status tx-validation` 
- **Request Logs**: `journalctl -u tx-validation -f`
- **Health Endpoint**: `curl http://127.0.0.1:18000/health`

## 🎯 **Integration with Rust Code**

The service is designed to work seamlessly with Rust validation code:

```rust
// This will work automatically once service is installed
let result = validate_state_changes_against_python(tx_hash, rust_changes).await;
```

The Rust client defaults to `http://127.0.0.1:18000` for all validation requests.

This setup ensures the Python validation service is always available for your analytics pipeline without any manual intervention.