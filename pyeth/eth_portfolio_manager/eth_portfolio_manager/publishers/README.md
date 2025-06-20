# Token Information Publishers

## Overview

The publishers module extracts and publishes critical token and pool information from the Python blockchain processor to the Rust mempool analyzer. This data enables real-time transaction impact assessment and scam detection.

## Architecture

```
Token Updates (Python)  →  TokenInfoExtractor  →  TokenInfoPublisher  →  ZMQ  →  Rust Mempool Processor
```

## Published Data Schema

### Core Token Information

```python
{
    "token_address": "0x...",
    "token_symbol": "SYMBOL",
    "token_name": "Token Name",
    "token_decimals": 18,
    "total_supply": "1000000000000000000000000",
    
    # Contract Features
    "buy_tax": 0.05,          # 5% buy tax
    "sell_tax": 0.05,         # 5% sell tax
    "max_tx_amount": "1000000000000000000",  # Max tokens per tx
    "max_wallet_amount": "2000000000000000000",  # Max tokens per wallet
    "trading_enabled": true,
    
    # Risk Flags
    "is_honeypot": false,     # Can't sell after buying
    "owner_can_pause": true,  # Owner can disable trading
    "owner_can_change_tax": true,  # Owner can modify taxes
    "liquidity_locked": false,
    "contract_verified": true,
    
    # Activity Metrics
    "holder_count": 150,
    "unique_traders_24h": 89,
    "volume_24h_eth": 125.5
}
```

### Pool Information (Per Pool)

```python
{
    "pool_address": "0x...",
    "pool_type": "V2",        # V2, V3, V4
    "fee_tier": 3000,         # 0.3% for V2/V3
    
    # Liquidity State
    "eth_reserve": 123.45,
    "token_reserve": 1000000.0,
    "k_value": 123450000.0,   # Constant product
    "liquidity_usd": 250000.0,
    
    # Recent Activity
    "last_trade_block": 19234567,
    "last_trade_timestamp": 1234567890,
    "trades_last_100_blocks": 45,
    "unique_traders_100_blocks": 23,
    
    # Price & Impact
    "current_price_eth": 0.00012345,
    "price_change_24h": -0.15,  # -15%
    "reserve_change_10_blocks": -5.2,  # ETH change
    
    # Concentration Metrics
    "is_primary_pool": true,  # Has most volume
    "pool_dominance": 0.85,   # 85% of total liquidity
}
```

## Key Data Points for Mempool Analysis

### 1. **Tax Impact Calculation**
```python
# When mempool sees: swap 1 ETH for TOKEN
actual_tokens_received = swap_output * (1 - buy_tax)
actual_eth_received = swap_output * (1 - sell_tax)
```

### 2. **Transaction Validation**
```python
# Will transaction succeed?
if not trading_enabled:
    return "WILL_REVERT: Trading disabled"
if amount > max_tx_amount:
    return "WILL_REVERT: Exceeds max transaction"
if recipient_balance + amount > max_wallet_amount:
    return "WILL_REVERT: Exceeds max wallet"
```

### 3. **Price Impact Assessment**
```python
# For large trades
price_impact = calculate_price_impact(
    eth_amount, 
    eth_reserve, 
    token_reserve,
    fee_tier
)
if price_impact > 0.10:  # 10% impact
    return "HIGH_IMPACT_TRADE"
```

### 4. **Scam Detection Signals**
```python
# Red flags for Rust to evaluate
if is_honeypot and operation == "BUY":
    return "HONEYPOT_WARNING"
if owner_can_change_tax and recent_owner_activity:
    return "TAX_CHANGE_RISK"
if not liquidity_locked and liquidity < 1.0:
    return "RUG_PULL_RISK"
```

## Data Extraction Sources

### From Token Contract
- **Taxes**: `buyTax()`, `sellTax()`, `_taxRate`, etc.
- **Limits**: `_maxTxAmount`, `_maxWalletSize`
- **State**: `tradingEnabled`, `swapEnabled`
- **Ownership**: `owner()`, `renounceOwnership()` status

### From Pool Contract
- **Reserves**: `getReserves()`, `slot0()` for V3
- **Activity**: Transaction logs filtered by pool address
- **Liquidity**: `totalSupply()` of LP tokens

### From Transaction History
- **Trading Patterns**: Recent swaps, liquidity changes
- **Holder Analysis**: Distribution, whale movements
- **Owner Actions**: Tax changes, trading toggles

## Update Frequency

- **Real-time Updates**: On every block with token/pool changes
- **Batch Updates**: Full state sync every N blocks
- **Critical Updates**: Immediate push for:
  - Trading enabled/disabled
  - Tax rate changes
  - Large liquidity movements
  - Owner privilege changes

## Performance Considerations

1. **Message Size**: Target < 1KB per update
2. **Update Rate**: ~100-500 tokens/sec during peak
3. **Compression**: Optional zlib for batch updates
4. **Caching**: Rust side maintains state between updates

## Future Enhancements

1. **MEV Metrics**: Sandwich attack frequency, arbitrage volume
2. **Social Signals**: Holder growth rate, community metrics  
3. **Cross-pool Arbitrage**: Price discrepancies between pools
4. **Advanced Risk Scoring**: ML-based scam probability

## Integration Example

```python
# Python side - Publishing updates
token_info = {
    'token_address': token_address,
    'pools': {
        pool_addr: {
            'eth_reserve': 125.5,
            'buy_tax': 0.05,
            'trading_enabled': True,
            # ... other fields
        }
    }
}
await publisher.update_token_info({token_address: token_info})

# Rust side - Receiving updates
match msg_type {
    "token_updates" => {
        for (token_addr, token_data) in updates {
            // Update local cache
            token_cache.update(token_addr, token_data);
            
            // Check pending transactions
            for pending_tx in mempool.get_txs_for_token(token_addr) {
                let impact = calculate_impact(pending_tx, token_data);
                if impact.is_dangerous() {
                    alert_system.notify(pending_tx, impact);
                }
            }
        }
    }
}
```