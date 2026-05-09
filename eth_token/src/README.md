# eth_token/src

Local operating map for token-state modules. This crate consumes processed Rust
transaction/block facts and should not recreate tracing or decoding logic.

## Owns

| Module | Owns |
| --- | --- |
| `erc20/` | Token metadata, token snapshots, and token-level chain data helpers. |
| `pools/` | AMM pool state machines and pool-specific calculations. |
| `state/` | Token transfer state, control-address tracking, and pool-state bridges. |
| `health/` | Scam, volume, and trading-health scoring. |
| `network/` | Token address activity, graph construction, and snapshots. |
| `manager/` | Block-level orchestration over processed Rust transactions. |
| `tracking/` | Replay contexts, builders, and transaction application. |
| `chain_metadata/` | Chain metadata lookup and cache helpers. |
| `utils/` | Generic helpers with no domain ownership. |

## Does Not Own

- Receipt/log/trace decoding. Add decoded facts to `tx_processor` first.
- Direct Reth DB queries except through owner crate APIs.
- HTTP DTOs or runtime hosting; use `eth_token_server`.

## Data Flow

```text
tx_processor::ProcessedBlock
  -> manager/tracking applies txs in block order
  -> erc20 + pools + state + health + network updates
  -> eth_token_server view DTOs
```

## Current Focus

- Active parity path: ERC-20 plus Uniswap V2 token/pool tracking.
- Highest-traffic modules: `erc20`, `pools::uniswap::v2`, `state`,
  `tracking`, and `manager`.
- `health` and `network` should consume stabilized token/pool facts; do not
  move core pool lifecycle logic there.
