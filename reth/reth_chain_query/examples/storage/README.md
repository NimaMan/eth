# Storage Analysis Examples

This directory contains examples demonstrating direct contract storage analysis using reth_chain_query.

## Examples

### `uniswap_pool_state.rs`
Analyzes Uniswap V3 pool storage slots to extract real-time pool state including:
- Current liquidity
- Price tick and sqrt price
- Fee growth globals
- Protocol fees
- Pool slot0 packed data

### `erc20_storage_layout.rs`  
Demonstrates reading ERC20 token storage directly:
- Total supply from storage slot 2
- Balance mappings (slot 0 + keccak256(address, 0))
- Allowance mappings (slot 1 + keccak256(spender, keccak256(owner, 1)))
- Comparing storage reads vs view function calls for performance

### `proxy_implementation_detector.rs`
Detects proxy implementations by reading storage slots:
- EIP-1967 proxy storage slots (implementation at 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc)
- OpenZeppelin proxy patterns
- Minimal proxy (EIP-1167) detection
- Upgradeable contract analysis

## Performance Benefits

Direct storage access provides:
- **Sub-millisecond queries**: Read storage slots in ~0.1-0.5ms vs 50-200ms RPC calls
- **Atomic consistency**: All reads from same block height guaranteed
- **No rate limits**: Direct database access without RPC throttling
- **Batch efficiency**: Query multiple slots simultaneously

## Use Cases

Storage analysis is ideal for:
- DEX pool monitoring and arbitrage
- Token contract analysis and auditing
- Proxy pattern detection and upgradeability analysis  
- Historical state reconstruction
- MEV bot development
- DeFi protocol state monitoring