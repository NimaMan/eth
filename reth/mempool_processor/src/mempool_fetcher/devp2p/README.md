# DevP2P Method

## Overview

The DevP2P method implements Ethereum's native peer-to-peer protocol to receive transaction announcements directly from other nodes. This bypasses HTTP/WebSocket layers for lower latency.

## Performance Characteristics

### Target Performance

| Metric | Target | Status |
|--------|--------|--------|
| **Detection latency** | <10ms | In development |
| **P95 latency** | <10ms | Target |
| **Mempool coverage** | 100% | Via P2P gossip |
| **Implementation** | 60% | Framework complete |

### Expected Benefits

- Direct protocol access (no JSON-RPC overhead)
- Peer-to-peer transaction propagation
- Lower latency than WebSocket
- Network topology awareness

## Current Status

The DevP2P implementation is partially complete:

✅ **Implemented:**
- RLPx protocol handshake
- ETH protocol negotiation
- Message encoding/decoding
- Basic peer connection

🚧 **In Progress:**
- Transaction pool synchronization
- Robust error handling
- Peer discovery
- Connection management

## Protocol Flow

1. **Connect** to Ethereum peer on port 30303
2. **Handshake** using RLPx encrypted protocol
3. **Negotiate** ETH protocol version
4. **Subscribe** to NewPooledTransactionHashes
5. **Request** full transactions via GetPooledTransactions
6. **Process** transactions immediately

## Usage (When Complete)

```rust
use mempool_processor::mempool_fetcher::devp2p::DevP2pClient;

// Connect to peer
let client = DevP2pClient::new("127.0.0.1:30303").await?;

// Start monitoring
client.start_monitoring().await?;

// Get transactions
loop {
    let transactions = client.fetch_new_transactions().await?;
    for tx in transactions {
        // Process with <10ms latency
    }
}
```

## Requirements

- Direct connection to Ethereum P2P network
- Port 30303 access
- Node ID and capabilities negotiation
- Encryption keys for RLPx

## When to Use

✅ **Use DevP2P when:**
- Need <10ms detection latency
- Have direct P2P network access
- Building high-frequency trading systems
- Network topology matters

❌ **Don't use DevP2P when:**
- Behind restrictive firewalls
- Need simple integration
- WebSocket latency is sufficient

## Advantages

1. **Low Latency**: Direct protocol access
2. **Network Native**: How transactions actually propagate
3. **No Middleman**: Skip RPC/WebSocket layers
4. **Topology Aware**: Choose optimal peers

## Limitations

- Complex implementation
- Requires P2P network access
- Currently incomplete (60%)
- Higher maintenance overhead