# Token Tracking Examples

This directory contains examples demonstrating the token tracking cache functionality used in the mempool processor.

## Examples

### 1. `test_token_cache.rs`
Comprehensive test suite for the `AddressTrackingCache` that demonstrates:
- Basic cache creation and initialization
- Adding token, creator, owner, and pool data
- Querying cache for different entity types (creators, owners, pools)
- Recording function calls from specific addresses
- Testing cache growth with multiple tokens
- Retrieving and verifying cache statistics

**Use case**: Run this to verify the cache is working correctly and understand all available operations.

### 2. `zmq_connection_example.rs`
Shows how to connect to the Python token publisher via ZMQ:
- Connects to ZMQ endpoints (pub: 5557, rep: 5558)
- Requests initial token data from Python
- Processes token update messages
- Loads received data into the cache

**Use case**: Use this as a reference for integrating with the Python token publisher.

### 3. `address_role_detection_example.rs`
Demonstrates address role detection and cache lookups:
- Adding tokens with different creator/owner addresses
- Detecting address roles (creator vs owner)
- Checking if an address has special permissions
- Simulating transaction routing based on address roles

**Use case**: Understand how to detect creator transactions vs regular transactions.

## Running the Examples

These examples are not registered in Cargo.toml to keep them separate from the main examples. To run them:

```bash
# Run from the mempool_processor directory
rustc --edition 2021 examples/token_tracking/test_token_cache.rs \
  -L target/debug/deps \
  --extern mempool_processor=target/debug/libmempool_processor.rlib \
  --extern tokio=target/debug/deps/libtokio-*.rlib \
  --extern tracing=target/debug/deps/libtracing-*.rlib \
  -o target/debug/test_token_cache

./target/debug/test_token_cache
```

Or compile them as regular Rust files with appropriate dependencies.

## Token Cache Architecture

The token tracking system consists of:
- **AddressTrackingCache**: Main cache storing token/creator/owner relationships
- **TokenTrackingSubscriber**: ZMQ subscriber that connects to Python publisher
- **PoolStateCache**: Tracks pool ETH reserves and state
- **TokenCreatorCache**: Maps tokens to their creators

The Python publisher (running on port 5556 for signals, 5557/5558 for token data) sends:
- Initial token data on request
- Real-time updates for new tokens and pool changes
- Creator/owner updates when ownership changes