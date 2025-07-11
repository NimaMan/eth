# Production TX Processor Examples

This directory contains production-ready examples for real-world transaction analysis use cases.

## Examples Overview

### `arbitrage_detection.rs`
Comprehensive arbitrage opportunity detection and analysis.

**Features:**
- Cross-DEX arbitrage pattern recognition
- Profitability calculation with gas costs
- Trading strategy analysis
- Risk assessment scoring
- Real arbitrage transaction examples

**Use Cases:**
- MEV research and detection
- Trading strategy development
- Market efficiency analysis
- Arbitrage bot monitoring

### `mev_analysis.rs`
Maximal Extractable Value (MEV) detection and categorization.

**Features:**
- Sandwich attack detection
- Front-running identification
- Back-running analysis
- MEV profit calculation
- Block-level MEV extraction analysis

**Use Cases:**
- MEV research and monitoring
- Trading strategy protection
- Network security analysis
- Economic impact assessment

### `whale_tracking.rs`
Large holder movement analysis and impact assessment.

**Features:**
- Large transaction detection
- Whale address identification
- Market impact calculation
- Movement pattern analysis
- Portfolio change tracking

**Use Cases:**
- Market intelligence
- Whale movement alerts
- Portfolio management
- Risk assessment

### `token_launch_analysis.rs`
New token deployment and initial trading analysis.

**Features:**
- Token launch detection
- Initial liquidity analysis
- Early trading pattern identification
- Price discovery metrics
- Launch success indicators

**Use Cases:**
- Token launch monitoring
- Investment opportunity identification
- Market maker analysis
- Due diligence automation

### `scam_detection.rs`
Suspicious transaction pattern detection for security analysis.

**Features:**
- Honeypot detection
- Rug pull pattern identification
- Pump and dump scheme detection
- Suspicious token behavior analysis
- Risk scoring for new tokens

**Use Cases:**
- Security monitoring
- Investor protection
- Exchange due diligence
- Compliance monitoring

## Running Production Examples

### Prerequisites
- Rust toolchain with Cargo
- PostgreSQL database with historical data
- Optional: Local Ethereum node for real-time analysis

### Arbitrage Detection
```bash
# Analyze specific transaction
cargo run --example arbitrage_detection -- 0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006

# With custom profit threshold
cargo run --example arbitrage_detection -- --min-profit 1.0 --tx-hash 0x...

# Batch analysis mode
cargo run --example arbitrage_detection -- --batch-file arbitrage_txs.txt
```

### MEV Analysis
```bash
# Analyze MEV in specific block
cargo run --example mev_analysis -- --block-number 18000000

# Search for sandwich attacks
cargo run --example mev_analysis -- --strategy sandwich --victim 0x...

# Block range analysis
cargo run --example mev_analysis -- --start-block 18000000 --end-block 18001000
```

### Whale Tracking
```bash
# Track specific whale address
cargo run --example whale_tracking -- --address 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045

# Set minimum transaction size
cargo run --example whale_tracking -- --min-value 100.0 --address 0x...

# Monitor multiple addresses
cargo run --example whale_tracking -- --address-file whale_addresses.txt
```

### Token Launch Analysis
```bash
# Analyze recent token launches
cargo run --example token_launch_analysis -- --recent 24h

# Analyze specific token launch
cargo run --example token_launch_analysis -- --token 0x... --launch-block 18000000

# Market maker analysis
cargo run --example token_launch_analysis -- --focus market-makers --token 0x...
```

### Scam Detection
```bash
# Analyze suspicious token
cargo run --example scam_detection -- --token 0x... 

# Honeypot detection
cargo run --example scam_detection -- --type honeypot --token 0x...

# Batch scam analysis
cargo run --example scam_detection -- --token-file suspicious_tokens.txt
```

## Configuration

### Environment Variables
```bash
# Database connection
export DATABASE_URL="postgresql://user:pass@localhost/qarqa"

# RPC endpoints
export ETH_RPC_URL="http://localhost:8545"
export ETH_WS_URL="ws://localhost:8546"

# API keys (if using external services)
export ETHERSCAN_API_KEY="your_key_here"
export COINGECKO_API_KEY="your_key_here"
```

### Configuration Files
Many examples support JSON configuration files:

```json
{
  "arbitrage_config": {
    "min_profit_eth": 0.1,
    "max_hops": 4,
    "include_flash_loans": true,
    "dex_whitelist": ["uniswap_v2", "uniswap_v3", "sushiswap"]
  },
  "whale_config": {
    "min_transaction_eth": 100.0,
    "whale_addresses": ["0x...", "0x..."],
    "alert_thresholds": {
      "eth": 1000.0,
      "usdc": 1000000.0
    }
  }
}
```

## Real-World Data

All examples use production data sources:
- **Real transaction hashes** from mainnet
- **Known whale addresses** and trading patterns
- **Documented MEV transactions** for validation
- **Historical arbitrage opportunities** for testing

## Performance Considerations

### Memory Usage
- **Arbitrage detection**: ~50MB per 1000 transactions
- **MEV analysis**: ~100MB per block (depends on transaction count)
- **Whale tracking**: ~10MB per address per day
- **Batch processing**: Linear scaling with input size

### Processing Speed
- **Single transaction analysis**: 100-500ms (with REVM)
- **Block analysis**: 5-30 seconds (depends on complexity)
- **Historical analysis**: 1-10 transactions per second
- **Real-time processing**: <1 second latency

### Optimization Tips
1. **Use batch processing** for multiple transactions
2. **Cache REVM state** for sequential analysis
3. **Filter transactions** by value/gas before analysis
4. **Parallelize** independent transaction analysis

## Integration Examples

### With Database
```rust
// Historical arbitrage analysis
let arbitrage_detector = ArbitrageDetector::new()
    .with_database_connection(&db_pool)
    .with_historical_lookback(Duration::hours(24));

let opportunities = arbitrage_detector
    .scan_historical_transactions()
    .await?;
```

### With Network Building
```rust
// Build arbitrage network
let fund_flows = arbitrage_detector.analyze_transaction(&tx).await?;
let network = NetworkBuilder::new()
    .build_from_fund_flows(&fund_flows, None)?;
```

### With Live Processing
```rust
// Real-time MEV monitoring
let mev_monitor = MevAnalyzer::new()
    .with_live_mempool_feed(&ws_connection)
    .with_alert_callback(|mev| {
        println!("MEV detected: {:.2} ETH profit", mev.profit_eth);
    });

mev_monitor.start_monitoring().await?;
```

## Output Formats

### JSON Output
```bash
cargo run --example arbitrage_detection -- --output-format json > results.json
```

### CSV Output
```bash
cargo run --example whale_tracking -- --output-format csv > whale_movements.csv
```

### Database Storage
```bash
cargo run --example mev_analysis -- --store-results --database-url postgresql://...
```

## Monitoring and Alerts

### Alert Integration
Examples support webhook alerts for significant findings:

```rust
let webhook_url = "https://discord.com/api/webhooks/...";
arbitrage_detector.set_alert_webhook(webhook_url);
```

### Metrics Export
Performance and detection metrics can be exported:

```rust
let metrics = arbitrage_detector.get_metrics();
println!("Detection rate: {:.2}%", metrics.detection_rate);
println!("False positives: {}", metrics.false_positives);
```

## Validation and Testing

### Accuracy Validation
All detection algorithms include validation against known examples:

```bash
# Run validation suite
cargo run --example arbitrage_detection -- --validate

# Test against known datasets
cargo run --example mev_analysis -- --test-dataset known_mev_transactions.json
```

### Performance Benchmarks
```bash
# Benchmark detection speed
cargo run --example arbitrage_detection -- --benchmark

# Memory usage profiling
cargo run --example whale_tracking -- --profile-memory
```

## Contributing

When adding new production examples:
1. Use real transaction data for testing
2. Include comprehensive error handling
3. Add performance benchmarks
4. Document configuration options
5. Include validation against known results
6. Add integration tests
7. Update this README