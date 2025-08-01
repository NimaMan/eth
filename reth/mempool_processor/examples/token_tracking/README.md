# Token Tracking Examples

This directory contains examples demonstrating the token tracking cache functionality used in the mempool processor.

## Examples

### 1. `inspect_creator_info.rs`
Comprehensive creator and token inspection tool that:
- Looks up creator addresses in the cache
- Displays all tokens created by a specific address
- Shows detailed token metadata, pools, and simulation results
- Can analyze token/pool/creator addresses with --detailed flag
- Demonstrates how to get complete token information for routing

**Use case**: Debug creator transactions and inspect token data comprehensively.

### 2. `get_token_info.rs`
Quick token information retrieval tool that:
- Gets token info by token address, creator, or pool
- Shows summary information suitable for the router
- Displays primary pool liquidity and trading status
- Demonstrates efficient token cache queries

**Use case**: Quick lookups for routing decisions and token verification.

### 3. `token_cache_inspector.rs`
Shows token cache statistics and contents:
- Displays total tokens, pools, and creators in cache
- Lists sample tokens with their metadata
- Shows cache initialization from ZMQ data

**Use case**: Monitor cache health and verify data population.

### 4. `zmq_connection_example.rs`
Shows how to connect to the Python token publisher via ZMQ:
- Connects to ZMQ endpoints (pub: 5557, rep: 5558)
- Requests initial token data from Python
- Processes token update messages
- Loads received data into the cache

**Use case**: Use this as a reference for integrating with the Python token publisher.

## Running the Examples

All examples are registered in Cargo.toml and can be run with:

```bash
# Inspect creator and token information
cargo run --example inspect_creator_info -- --creator 0xYourCreatorAddress

# Get quick token info
cargo run --example get_token_info -- --token 0xTokenAddress
# Or by creator
cargo run --example get_token_info -- --creator 0xCreatorAddress
# Or by pool
cargo run --example get_token_info -- --pool 0xPoolAddress

# Inspect cache statistics
cargo run --example token_cache_inspector

# Test ZMQ connection
cargo run --example zmq_connection_example
```

## Token Cache Architecture

The token tracking system consists of:
- **TokenTrackingCache**: Unified cache storing tokens, creators, and pool relationships
- **TokenTrackingSubscriber**: ZMQ subscriber that connects to Python publisher
- **PoolStateCache**: Tracks pool ETH reserves and state

The Python publisher (running on port 5556 for signals, 5557/5558 for token data) sends:
- Initial token data on request
- Real-time updates for new tokens and pool changes
- Creator/owner updates when ownership changes