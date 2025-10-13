# Entity Query Scripts

This directory contains example scripts demonstrating how to query CEX, ETF, and Stablecoin data from the Ethereum blockchain using the high-performance PyReth backend.

## Available Scripts

### 📊 Stablecoins
- `stablecoins/query_stablecoins_with_pyreth.py` - Get stablecoin supplies, balances, and market data
- `stablecoins/stablecoin_market_by_unit.py` - Analyze stablecoin market by unit of account

### 🏦 CEX (Centralized Exchanges)
- `cex/query_cex_flows.py` - Track exchange balances, deposits, withdrawals, and market concentration

### 💼 ETF (Exchange-Traded Funds)
- `etf/query_etf_holdings.py` - Monitor ETF provider holdings, flows, and institutional movements

### 🎯 Combined Dashboard
- `dashboard_example.py` - Aggregate all three data sources into a comprehensive market overview

## Installation

Ensure you have the required dependencies:
```bash
# Activate the qw conda environment
conda activate qw

# Set Python path if needed
export PYTHONPATH=/home/nima/code/crypto/py:$PYTHONPATH
```

## Usage

### Individual Scripts

```bash
# Get stablecoin market data
python stablecoins/query_stablecoins_with_pyreth.py

# Analyze CEX flows
python cex/query_cex_flows.py

# Monitor ETF holdings
python etf/query_etf_holdings.py

# Get complete dashboard data
python dashboard_example.py
```

### Import in Python

```python
from eth_data.database.reth_chain_queries.entities.stablecoin_queries import StablecoinQueries
from eth_data.database.reth_chain_queries.entities.cex_queries import CEXQueries
from eth_data.database.reth_chain_queries.entities.etf_queries import ETFQueries

# Initialize queries (they share PyReth instance automatically)
stablecoin_q = StablecoinQueries()
cex_q = CEXQueries()
etf_q = ETFQueries()

# Get dashboard-ready data
stablecoin_overview = stablecoin_q.get_market_overview(hours_back=24)
cex_flows = cex_q.get_exchange_flows(hours_back=24)
etf_flows = etf_q.get_flows_summary(hours_back=24)
```

## Key Features

### CEX Queries
- **Current Balances**: Get ETH holdings for 335+ exchanges
- **Flow Analysis**: Track deposits/withdrawals over any time period
- **Market Concentration**: Calculate HHI index and concentration risk
- **Exchange Patterns**: Analyze deposit/withdrawal patterns per exchange
- **Historical Tracking**: Monitor balance changes over time

### ETF Queries  
- **Provider Holdings**: Track BlackRock, Grayscale, Fidelity, and others
- **Flow Monitoring**: Detect institutional inflows/outflows
- **AUM Calculation**: Convert ETH holdings to USD values
- **Whale Detection**: Identify large transfers (>1000 ETH)
- **Cumulative Analysis**: Track cumulative flows over time

### Stablecoin Queries
- **Supply Tracking**: Monitor USDC, USDT, DAI, and other stablecoins
- **Balance Analysis**: Check stablecoin holdings for any address
- **Supply Changes**: Track minting/burning between blocks
- **Market Overview**: Get complete stablecoin market metrics

## Output Formats

All scripts provide:
1. **Human-readable console output** with formatted tables
2. **JSON export** for API integration
3. **Dashboard-ready data structures**

## Performance

Using PyReth (Rust backend) provides:
- **91.5x faster** than Python RPC calls
- Sub-second query times for complex aggregations
- Real-time data from local Reth node

## Architecture

```
PyReth (Rust) → ChainQuery → Entity Queries → Scripts
     ↑                           ↑              ↑
  Singleton                 Type-safe      Examples
  Instance                  Wrappers       & Tests
```

## Common Use Cases

1. **Dashboard Data**: Use `dashboard_example.py` for Ethereum Today page
2. **Exchange Monitoring**: Track CEX flows to detect market sentiment
3. **Institutional Flows**: Monitor ETF movements for whale activity  
4. **Stablecoin Health**: Detect expansion/contraction in stablecoin supply
5. **Market Analysis**: Combine all three for comprehensive market view

## Troubleshooting

If you encounter import errors:
```bash
# Ensure you're in the correct directory
cd /home/nima/code/crypto/py/eth_data

# Check Python path
export PYTHONPATH=/home/nima/code/crypto/py:$PYTHONPATH

# Use conda environment
conda activate qw
```

## Data Sources

All data comes from:
- Local Reth node (http://localhost:8545)
- PostgreSQL database (eth_db)
- PyReth singleton instance (shared across queries)

## Note on Stablecoin Data

If stablecoin queries return zero values, ensure:
1. PyReth is properly initialized
2. The Reth node is fully synced
3. Token addresses are correctly configured in `stablecoin_addresses.py`