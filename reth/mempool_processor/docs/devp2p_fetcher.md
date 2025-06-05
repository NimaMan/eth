# DevP2P Transaction Fetcher

## Objective
**Get Ethereum transactions immediately when they enter the network, bypassing RPC polling delays to achieve sub-10ms transaction arrival times for real-time scam detection.**

## Current vs DevP2P Performance

### Current RPC Fetcher Performance:
- **Queue Time:** 12-92ms (polling delay + network latency)
- **Method:** HTTP RPC polling every 50ms
- **Bottleneck:** Polling interval + RPC processing time

### DevP2P Fetcher Target Performance:
- **Queue Time:** <10ms (direct peer-to-peer notification)
- **Method:** Real-time transaction broadcast subscription
- **Advantage:** Immediate notification when transactions hit network

## DevP2P Protocol Overview

### What is DevP2P?
DevP2P is Ethereum's peer-to-peer networking protocol that nodes use to communicate directly. It allows us to:

1. **Connect as an Ethereum network peer**
2. **Subscribe to transaction announcements** (`NewPooledTransactionHashes`)
3. **Request transaction details** (`GetPooledTransactions`) 
4. **Receive transactions immediately** when broadcasted

### Protocol Messages:
- `NewPooledTransactionHashes` - Notification of new transactions
- `GetPooledTransactions` - Request full transaction data
- `PooledTransactions` - Response with transaction details

## Implementation Architecture

### Core Components:

1. **DevP2pClient** - Core peer-to-peer connection
2. **TransactionSubscriber** - Handles transaction announcements
3. **TxPoolManager** - Manages transaction requests/responses
4. **DevP2pFetcher** - Implements TransactionSource trait

### Flow Diagram:
```
Ethereum Network Peer → DevP2P Connection → Transaction Announcement 
                                                       ↓
Transaction Hash Received → Request Full Transaction → Process Transaction
                                                       ↓
Add to Queue → REVM Simulation → Scam Detection
```

## Speed Comparison

### Transaction Arrival Timeline:

**Current RPC Method:**
```
T=0ms:   Transaction broadcasted to network
T=0-50ms: Waiting for next polling cycle
T=50ms:   RPC request sent
T=70ms:   RPC response received
T=92ms:   Transaction processed
```

**DevP2P Method:**
```
T=0ms:    Transaction broadcasted to network
T=1-3ms:  DevP2P peer notification received
T=4-6ms:  Request full transaction data
T=7-10ms: Transaction processed
```

**Speed Improvement: 9x faster (92ms → 10ms)**

## Implementation Status

### Current Code State:
- ✅ `FetchMode::DevP2p` enum variant defined
- ✅ `get_transactions_devp2p()` method implemented with framework
- ✅ DevP2P client module created (`devp2p_client.rs`)
- ✅ Command-line option `--enable-devp2p` added
- ⚠️  **Dependency compatibility issues:** devp2p crate incompatible with modern secp256k1
- 🔧 **Alternative approach needed:** Use reth networking or custom implementation

### Current Working Test:
To test the DevP2P framework (falls back to RPC currently):

```bash
# Test DevP2P mode (currently falls back to enhanced RPC)
./target/release/scam_detection_service \
    --eth-rpc-url http://localhost:8545 \
    --pool-zmq-address tcp://localhost:5557 \
    --enable-devp2p \
    --verbose
```

**Expected output:**
```
🚀 DevP2P mode enabled - targeting <10ms transaction arrival
🔗 DevP2P fetcher activated - Direct peer-to-peer transaction fetching
🔌 DevP2P client creation failed: [...], falling back to RPC
📡 Using RPC fallback (target: upgrade to DevP2P for <10ms latency)
```

### Dependency Resolution Options:

#### Option 1: Use Reth Networking (Recommended)
```toml
# Replace devp2p with reth networking
reth-network = "0.1"
reth-primitives = "0.1"
```

#### Option 2: Custom DevP2P Implementation
- Implement minimal DevP2P subset for transaction announcements
- Use modern secp256k1 v0.29+ directly
- Focus only on ETH protocol transaction pool messages

#### Option 3: Alternative Fast Access
- Use Ethereum node's admin API for peer connections
- WebSocket subscription to new pending transactions
- IPC connection to node's transaction pool

## Benefits for Scam Detection

### Why Speed Matters:
1. **Earlier Detection:** Catch scams before they execute
2. **Faster Alerts:** More time to warn users/protocols
3. **Better Prevention:** Potential to front-run malicious transactions
4. **Reduced False Positives:** More accurate timing analysis

### Expected Performance Gains:
- **Queue Time:** 92ms → 10ms (9x improvement)
- **End-to-End:** 92ms → 10ms (9x improvement) 
- **SLA Compliance:** From 4.5% → 95%+ (100ms target achievable)
- **Real-time Capability:** True real-time scam detection

## Next Steps

1. **Implement DevP2pClient** - Core peer connection
2. **Add Transaction Subscription** - Listen for new tx announcements
3. **Integrate with Fetcher** - Complete `get_transactions_devp2p()`
4. **Performance Testing** - Validate <10ms queue times
5. **Production Deployment** - Replace RPC polling with DevP2P

## Usage

Once implemented, enable DevP2P fetching:

```rust
let fetcher = MempoolFetcher::with_options(
    "http://localhost:8545",  // Fallback RPC
    5000,                     // Cache size
    true,                     // Batch requests
    250,                      // Batch size
    1000,                     // Timeout
    FetchMode::DevP2p         // Use DevP2P instead of RPC
)?;
```

This will reduce transaction arrival latency from ~92ms to <10ms, achieving true real-time scam detection capability.

# Enabling the **dev‑p2p** Fetch Path
These instructions capture, step‑by‑step, how to replace the JSON‑RPC batching approach with a *true* dev‑p2p listener powered by **reth**.  Follow them in order; every step should compile before you move on.

---

## 1 Add the published reth crates

```toml
# Cargo.toml ── [dependencies]
reth-ipc              = "1.4"
reth-transaction-pool = "1.4"
reth-primitives       = "19.0"      # already brought by revm but we pin it
```

*No feature flags are required—the IPC & TxPool APIs are exported directly.*

---

## 2 Run a local reth node with IPC

```bash
reth node --max-peers 100 --ipc --ipc-path /tmp/reth.ipc
```

* Keep it on the same host as the bot for sub‑20 ms delivery.

---

## 3 Define the `FetchMode` enum

```rust
/// How we fetch mempool transactions
#[derive(Clone, Copy, Debug)]
pub enum FetchMode {
    RpcBatch,
    RpcSingle,
    DevP2p,
}
```

Add a `fetch_mode: FetchMode` field to `MempoolFetcher` and default it to `DevP2p` in `new()`.

---

## 4 Implement the dev‑p2p listener

```rust
use reth_ipc::TxPoolIpc;
use reth_transaction_pool::{FullTxEvent, TransactionPool};
use reth_primitives::{TransactionSigned};

async fn get_transactions_devp2p(&self) -> eyre::Result<Vec<TransactionView>> {
    // ❶ Connect over the same Unix socket the node exposed
    let ipc = TxPoolIpc::connect("/tmp/reth.ipc").await?;

    // ❷ Stream TxPool events
    let mut stream = ipc.events();
    let mut out = Vec::new();

    while let Some(Ok(FullTxEvent::NewTransaction(tx))) = stream.next().await {
        out.push(TransactionView::from(&tx));
        if out.len() >= self.max_batch_size { break; }
    }
    Ok(out)
}
```

Add a simple `impl From<&TransactionSigned> for TransactionView` (hash/from/to/value/… extraction).

---

## 5 Wire it into the fetch selector

```rust
match self.fetch_mode {
    FetchMode::DevP2p   => return self.get_transactions_devp2p().await,
    FetchMode::RpcBatch => return self.get_transactions_batch().await,
    FetchMode::RpcSingle=> { /* existing non‑batch path */ }
}
```

---

## 6 Expose the CLI flag

```rust
#[arg(long, default_value = "devp2p")]
fetch_mode: String,
```

Parse into the enum in `performance_monitor.rs`.

---

## 7 Measure the real latency

Insert a timestamp **immediately** when a `FullTxEvent` arrives and subtract it from `Instant::now()` when you push the TX into the processing queue:

```rust
let seen = Instant::now();
/* … simulation … */
metric.fetch_time_ms = seen.elapsed().as_millis() as u64;
```

Expected output:

```
Fetch Time (ms) : Avg ≈ 15–30
Simulation Time: Avg ≈ 0.06
Total           : < 40 ms
```

---

## 8 Fallback testing

Run three benchmarks:

```bash
# dev‑p2p (default)
cargo run --release --bin performance_monitor --tx-count 500

# rpc‑batch
a)   --fetch-mode rpc_batch
# rpc‑single
b)   --fetch-mode rpc_single
```

You should see `devp2p` outperform both by \~10×.

---

## 9 Troubleshooting

| Symptom                    | Check                                                                           |
| -------------------------- | ------------------------------------------------------------------------------- |
| `TxPoolIpc::connect` hangs | Is `/tmp/reth.ipc` created and readable by the bot user?                        |
| No events delivered        | Does the node have at least \~45 peers? (`reth node --metrics`)                 |
| Latency > 100 ms           | The node is remote or connected via TCP; keep bot and node on the same machine. |

---

## 10 Next after dev‑p2p

* Remove JSON‑RPC from the hot path entirely.
* Benchmark again and update the dev‑plan doc with the new numbers.
* Begin work on Phase 2 bridge (REQ/REP reserve fetch).

---

### Commit checklist

* [ ] `Cargo.toml` updated with three `reth-*` crates
* [ ] `FetchMode::DevP2p` implemented
* [ ] CLI default set to `devp2p`
* [ ] Performance monitor prints sub‑30 ms fetch
