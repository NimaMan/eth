# TxBuilder Examples

Examples demonstrating user-friendly transaction construction using ethtx.TxBuilder.

## Examples

### 01_basic_transaction_building.py
Build common Ethereum transactions without ABI knowledge:
- ETH transfers
- ERC20 transfers using token symbols
- ERC20 approvals (including unlimited)
- Token information queries
- Gas estimation

## Usage

```python
import ethtx

# Initialize builder for mainnet
builder = ethtx.TxBuilder.mainnet()

# ETH transfer
eth_tx = builder.eth_transfer(
    from_addr="0x...",
    to_addr="0x...",
    amount="1.5"  # 1.5 ETH
)

# ERC20 transfer using symbol
usdc_tx = builder.erc20_transfer(
    token="USDC",  # Symbol, not address!
    from_addr="0x...",
    to_addr="0x...",
    amount="1000.0"  # Handles decimals automatically
)

# ERC20 approval
approve_tx = builder.erc20_approve(
    token="USDC",
    owner="0x...",
    spender="uniswap_router",  # Can use protocol names
    amount="unlimited"  # Max uint256
)

# Get token info
info = builder.get_token_info("USDC")
# Returns: {"address": "0x...", "decimals": 6, "symbol": "USDC"}

# List available tokens
tokens = builder.list_tokens()
# Returns: ["USDC", "USDT", "WETH", "DAI", ...]
```

## Features

- **No ABI required**: Function signatures handled internally
- **Token registry**: 50+ common tokens pre-configured
- **Protocol names**: Use "uniswap_router" instead of addresses
- **Smart decimals**: Automatic decimal conversion
- **Amount parsing**: Supports "1.5", "unlimited", percentages
- **Gas estimation**: Built-in gas estimates for common operations