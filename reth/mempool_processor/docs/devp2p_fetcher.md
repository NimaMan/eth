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
