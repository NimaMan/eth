# ETH Kartal 🦅

> **Kartal** (Turkish: Eagle) - A swift predator that strikes from above

ETH Kartal is an automated trading protection system that responds to scam detection alerts by executing protective trades faster than malicious actors can drain liquidity pools.

## Overview

ETH Kartal integrates with the mempool processor's scam detection system to:
- Receive real-time alerts about potential scams via ZMQ
- Track token positions and balances
- Make intelligent decisions based on severity and drain percentage
- Execute protective trades through Uniswap V2
- Monitor execution with timeout protection

## Architecture

```
┌─────────────────────┐     ZMQ     ┌──────────────────┐
│ Mempool Processor   │─────5559────▶│  Alert Receiver  │
│ (Scam Detection)    │              └──────┬───────────┘
└─────────────────────┘                     │
                                           ▼
                                    ┌──────────────────┐
                                    │ Decision Engine  │◀──┐
                                    └──────┬───────────┘   │
                                           │               │
                      ┌────────────────────┼────────────┐  │
                      ▼                    ▼            ▼  │
              ┌──────────────┐    ┌──────────────┐    ┌───┴──────────┐
              │   Monitor    │    │ Partial Sell │    │Position Track│
              └──────────────┘    └──────┬───────┘    └──────────────┘
                                         │                    
                                         ▼                    
                                  ┌──────────────────────────────┐
                                  │   Transaction Builder         │
                                  │   (Uniswap V2 Router)        │
                                  └──────────────────────────────┘
```

## Current Implementation Status

### ✅ Phase 1 & 2 Complete
- **Alert Reception**: ZMQ subscriber with dedicated thread
- **Position Tracking**: ERC20 balance monitoring with 60s cache
- **Decision Engine**: Multi-tier response based on drain severity
- **Transaction Building**: Uniswap V2 integration with approval handling
- **Execution**: Full transaction lifecycle with monitoring

### 🚧 Upcoming Phases
- Phase 3: Risk & Safety (circuit breakers, daily limits)
- Phase 4: Performance (<200ms validation)
- Phase 5: Production Hardening

## Quick Start

### Prerequisites
- Rust 1.70+
- Running Reth node at `localhost:8545`
- Mempool processor with ZMQ publisher enabled
- Development wallet with ETH for gas

### Running ETH Kartal

1. Build the project:
```bash
cargo build --release
```

2. Run with basic configuration:
```bash
# Requires wallet address at minimum
./target/release/kartal --wallet-address 0xYOUR_WALLET_ADDRESS

# With full configuration
./target/release/kartal \
  --wallet-address 0xYOUR_WALLET \
  --rpc-url http://localhost:8545 \
  --alert-endpoint tcp://localhost:5559 \
  --slippage 0.05 \
  --test-mode  # For testing without real trades
```

3. Using environment variables:
```bash
export WALLET_ADDRESS=0xYOUR_WALLET
export ETH_RPC_URL=http://localhost:8545
export ALERT_ZMQ_ENDPOINT=tcp://localhost:5559
export RUST_LOG=info

./target/release/kartal
```

### Decision Logic

The system uses a multi-tier response strategy based on severity and drain percentage:

| Severity | Drain % | Action | Sell % | Description |
|----------|---------|--------|---------|-------------|
| Critical | ≥80% | Emergency Sell | 100% | Immediate full position exit |
| High | ≥50% | Partial Sell | 75% | Reduce exposure significantly |
| Medium | ≥25% | Small Sell | 25% | Take profits, reduce risk |
| Any | <25% | Monitor | 0% | Watch for further changes |

Additional factors:
- **Confidence threshold**: Minimum 80% confidence to act
- **Value threshold**: Minimum 0.01 ETH position value
- **Slippage protection**: Default 5% max price impact

## Modules

- **alert_processor/** - ZMQ alert reception with dedicated thread
- **strategy/** - Decision engine with configurable thresholds
- **tx_executor/** - Uniswap V2 transaction builder
- **wallet/** - Position tracking with balance caching

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