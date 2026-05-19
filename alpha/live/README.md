# Live

`alpha/live/` groups the confirmed-chain live infrastructure used by token tracking, mempool risk, and later trading runtimes.

The live system is the bridge between confirmed processed blocks, canonical token
state, token-server inspection, and mempool risk. The folder is split by
responsibility while keeping the existing crate names stable:

| Folder | Crate name | Role |
| --- | --- | --- |
| `state/` | `eth_live_state` | Shared live-state snapshot schemas and store traits. |
| `feed/` | `eth_live_feed` | Confirmed-chain feed contracts and runtime boundary over processed blocks and token updates. |
| `trading/` | `eth_live_trading` | Live strategy policy that turns token/pool/risk signals into explicit trade actions. |

`state/` is the shared protocol/read model. `feed/` is the writer/runtime side that can own confirmed live token updates. Token server and mempool runtimes can be hosted in one process now while still depending on these narrower crate boundaries.

## Runtime Model

The live token tracker is a continuous runtime, not a finite range runner. It
warms from recent processed blocks, then keeps applying every new confirmed
block handed to it by `eth_chain_server`'s `LiveChainRuntime`.

```text
eth_chain_server LiveChainRuntime
  -> execution RPC/WS new-head processing
  -> direct LiveBlockUpdate handoff

LiveTokenRuntime
  -> warmup from processed-block cache
  -> consume every new LiveBlockUpdate
  -> update tokens/pools/retention
  -> publish token update events

Token server API
  -> read live token state for Asena

Mempool processor
  -> read live token/pool/control-address state
  -> use LiveTxSimulator against latest live chain-state
  -> emit risk/signals
```

`LiveChainRuntime` is responsible for processing the confirmed block and handing
it to token tracking in order.

## Core Invariant

```text
LiveTokenRuntime is the only writer of canonical token/pool state.
Mempool processor and token server are readers/consumers.
```

`LiveBlockTokenProcessor` owns the canonical token and pool mutation path inside
`LiveTokenRuntime`. Mempool risk may read token, pool, creator, owner, tax setter,
and control-address context, but it must not mutate canonical token state.
Speculative mempool findings should be emitted as risk/signals.

The runtime applies blocks with one long-lived `LiveBlockTokenProcessor`.
Warmup and live tail use the same processor instance:

```text
load processed block
  -> lock live token state for mutation
  -> apply the block to the in-memory processor
  -> update progress/events
  -> unlock for readers
```

Do not clone the processor per block. The processor contains the token registry,
pool indexes, and token network graphs; cloning it in the warmup loop makes
startup cost grow with tracked state instead of with the block being applied.
Read endpoints may wait for the current block apply to finish. That is the
intended tradeoff until we add a separate snapshot publisher for heavy views.

## Hosted In One Process For Now

For the current implementation, `eth_chain_server` can host the live token
runtime, token-server API, and mempool runtime in one Rust process. That keeps
state sharing simple and avoids external process-boundary transports for the
token/mempool handoff.

The boundary should still stay modular:

- `feed/` defines the live token runtime and event contract.
- `state/` defines shared snapshots/readers/writers.
- `trading/` defines live trade policy before execution/broadcast wiring.
- `eth_chain_server` hosts the runtime and exposes HTTP views.
- `mempool_processor` consumes live token state and live token events.

Current deployment note: `trading/` now also contains the direct-raw tx-prep
boundary for Kartal, but the running alpha trader still uses no-capital
chain-state simulation. A real live strategy now needs runtime wiring around the
planner: a `LiveTxPlanningInputResolver`, live final simulation, gas-rank
provider, allowance reader, guarded real mode, and receipt reconciliation.

This lets us split the runtimes into separate services later without changing
the conceptual data flow.

## Related Docs

- `feed/README.md`: crate-level confirmed live-feed and token-runtime contract.
- `state/README.md`: shared live-state snapshots and store traits.
- `trading/README.md`: live LP approval priority-exit policy and deployment gates.
- `../../eth_chain_server/README.md`: chain-server API and inspector-facing live state exposure.
- `../../mempool_processor/README.md`: mempool risk processing, token context consumption, and `LiveTxSimulator` use.
