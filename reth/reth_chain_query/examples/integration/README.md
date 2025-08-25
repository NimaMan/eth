# Integration Examples

This directory contains comprehensive examples that combine multiple query types to demonstrate real-world blockchain analysis use cases.

## Examples

### `stablecoin_market_share.rs`
Comprehensive stablecoin market analysis combining:
- Token total supplies and metadata queries
- Whale holder balance analysis  
- Market cap calculations and rankings
- Performance comparison vs RPC methods
- Real-world replacement for the existing stablecoin market share API endpoint

### `etf_flow_analysis.rs`
ETF and institutional token flow analysis:
- Major ETF token holdings across multiple addresses
- Flow analysis between institutional wallets
- Liquidity concentration metrics
- Cross-token correlation analysis

### `whale_portfolio_tracker.rs`
Multi-token whale portfolio analysis:
- Track multiple token holdings for whale addresses
- Portfolio diversification metrics
- Historical balance trend analysis
- Risk assessment based on concentration

### `defi_protocol_health.rs`
DeFi protocol health monitoring:
- TVL calculation across multiple tokens
- Liquidity pool state analysis
- Protocol token distribution metrics
- Health score calculation based on multiple factors

## Performance Benefits

Integration examples demonstrate:
- **Batch Query Efficiency**: Multiple related queries in single database session
- **Atomic Consistency**: All data from same block height
- **Sub-second Analysis**: Complex multi-query analysis in milliseconds
- **Massive RPC Savings**: 100-1000x reduction in external API calls

## Use Cases

Perfect for:
- Replacing slow RPC-based dashboard APIs
- Real-time trading bot data feeds
- Institutional portfolio monitoring
- DeFi protocol risk assessment
- Market research and analytics