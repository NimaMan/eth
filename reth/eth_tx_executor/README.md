# ETH Kartal 🦅

> **Kartal** (Turkish: Eagle) - A swift predator that strikes from above

ETH Kartal is an automated trading protection system that responds to scam detection alerts by executing protective trades faster than malicious actors can drain liquidity pools.

## Overview

ETH Kartal integrates with the mempool processor's scam detection system to:
- Receive real-time alerts about potential scams
- Analyze market conditions and decide on protective strategies
- Execute trades to protect users from losses
- Monitor execution and track performance

## Architecture

```
Mempool Processor → ZMQ → Alert Processor → Strategy Engine → TX Executor
                                               ↓
                                         Risk Manager → Circuit Breaker
```

## Quick Start

### Prerequisites
- Rust 1.70+
- Running Reth node at `localhost:8545`
- Mempool processor with ZMQ publisher enabled
- Development wallet with ETH for gas

### Development Setup

1. Clone and navigate to the project:
```bash
cd /home/nima/code/crypto/rust/eth_kartal
```

2. Set up environment:
```bash
export ETH_KARTAL_PRIVATE_KEY_DEV="0x..." # Your dev wallet key
```

3. Start development environment:
```bash
./scripts/start_dev.sh
```

### Configuration

See `config/README.md` for detailed configuration options. Key files:
- `config/dev.toml` - Development settings
- `config/prod.toml` - Production settings

## Modules

- **alert_processor/** - Receives alerts from mempool processor
- **strategy/** - Decides how to respond to alerts
- **tx_executor/** - Builds and submits transactions
- **risk/** - Manages position limits and safety controls
- **wallet/** - Handles key management and signing
- **common/** - Shared types and utilities

Each module has its own README with detailed documentation.

## Development

### Building
```bash
cargo build --release
```

### Testing
```bash
cargo test
```

### Running
```bash
cargo run -- --config config/dev.toml
```

## Safety Features

1. **Circuit Breakers** - Automatic shutdown on excessive losses
2. **Position Limits** - Maximum exposure controls
3. **Simulation Required** - All trades simulated before execution
4. **Multi-RPC Redundancy** - Fallback submission endpoints
5. **Slippage Protection** - Maximum acceptable price impact

## Performance Targets

- Alert to decision: <50ms
- Decision to submission: <150ms
- Total response time: <200ms
- Success rate: >99%

## Monitoring

- Metrics endpoint: `http://localhost:9090/metrics`
- Health check: `http://localhost:8080/health`
- Logs: Configured via `log_file` in config

## Security

- Never commit private keys
- Use hardware wallets in production
- Validate all external inputs
- Implement rate limiting
- Monitor for unusual activity

## License

Proprietary - All rights reserved

## Contact

Nima Manaf - nima.manaf8@gmail.com