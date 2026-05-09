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
| `manager/` | Compatibility facade for older imports. |
| `tracking/` | Token registry state, tracked-token indexing, block update loop, and token update routing. |
| `chain_metadata/` | Chain metadata lookup and cache helpers. |
| `utils/` | Generic helpers with no domain ownership. |

## Does Not Own

- Receipt/log/trace decoding. Add decoded facts to `tx_processor` first.
- Direct Reth DB queries except through owner crate APIs.
- HTTP DTOs or runtime hosting; use `eth_token_server`.

## Data Flow

```text
tx_processor::ProcessedBlock
  -> tracking::block_processor applies txs in block order
  -> erc20 + pools + state + health + network updates
  -> eth_token_server view DTOs
```

## Block Simulation State Reuse

Trading simulation is block-scoped in the token pipeline. The range runner calls
`BlockTokenProcessor::process_block_with_discovery_provider`, which creates one
`PoolTradingSimulationMode::HistoricalBlockSession` per processed block. The first
pool simulation in that block opens a `BlockTxStateSession`; later V2/V3/V4
pool simulations in the same block reuse that session and branch from its
warmed prefix state.

```text
ProcessedBlock
  -> BlockTokenProcessor::process_block_with_discovery_provider
  -> ProcessedTokenUpdateRouter
  -> simulate_updated_v2/v3/v4_pools
  -> PoolTradingSimulationMode::HistoricalBlockSession
  -> TxSimulator::block_tx_state_session(block)
  -> advance one canonical tx prefix, then clone branches per pool simulation
```

Live token processing follows the same ownership rule, but uses
`LiveBlockSession` with a per-block `BTreeMap<u64, BlockStateSession>`.
The map is keyed by the effective simulation block number, so a live block can
reuse both parent-state and current-state sessions within that block.

The important invariant is that token/pool code should not call direct
per-pool historical simulation helpers when block-scoped session variants are
available. Historical range simulation should pay at most one block tx session
open per processed block, plus monotonic prefix replay to the highest simulated
tx index and cheap branch clones for individual pool checks.

## Current Focus

- Active parity path: ERC-20 plus known V2-router-compatible token/pool tracking.
- Highest-traffic modules: `erc20`, `pools::uniswap::v2`, `state`,
  and `tracking`.
- `health` and `network` should consume stabilized token/pool facts; do not
  move core pool lifecycle logic there.
