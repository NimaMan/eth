# Live

`alpha/live/` groups the confirmed-chain live infrastructure used by token tracking, mempool risk, and later trading runtimes.

The live system is the bridge between confirmed processed blocks, canonical token
state, token-server inspection, and mempool risk. The folder is split by
responsibility while keeping the existing crate names stable:

| Folder | Crate name | Role |
| --- | --- | --- |
| `state/` | `eth_live_state` | Shared live-state keys, snapshot schemas, and store traits. |
| `feed/` | `eth_live_feed` | Confirmed-chain feed contracts and runtime boundary over processed blocks and token updates. |

`state/` is the shared protocol/read model. `feed/` is the writer/runtime side that can own confirmed live token updates. Token server and mempool runtimes can be hosted in one process now while still depending on these narrower crate boundaries.

## Runtime Model

The live token tracker is a continuous runtime, not a finite range runner. It
warms from recent processed blocks, then keeps applying every new confirmed
block published by the live block processor.

```text
eth-live-block-processor
  -> processed block Redis/cache
  -> live chain-state overlay

LiveTokenRuntime
  -> warmup from processed-block cache
  -> consume every new processed block
  -> update tokens/pools/retention
  -> publish token update events

Token server API
  -> read live token state for Asena

Mempool processor
  -> read live token/pool/control-address state
  -> use LiveTxSimulator against latest live chain-state
  -> emit risk/signals
```

The live block processor is responsible for publishing the processed block and
advancing the live chain-state overlay first. After that, token tracking can use
the processed block as ordered confirmed input, while `LiveTxSimulator` and
`LivePoolBuySellSimulator` can target the latest tracked live state.

## Core Invariant

```text
LiveTokenRuntime is the only writer of canonical token/pool state.
Mempool processor and token server are readers/consumers.
```

`LiveBlockTokenProcessor` owns the canonical token and pool mutation path inside
`LiveTokenRuntime`. Mempool risk may read token, pool, creator, owner, tax setter,
and control-address context, but it must not mutate canonical token state.
Speculative mempool findings should be emitted as risk/signals.

## Hosted In One Process For Now

For the current implementation, `eth_token_server` can host the live token
runtime, token-server API, and mempool runtime in one Rust process. That keeps
state sharing simple and avoids Redis/ZMQ as an internal dependency for the
token/mempool handoff.

The boundary should still stay modular:

- `feed/` defines the live token runtime and event contract.
- `state/` defines shared snapshots/readers/writers.
- `eth_token_server` hosts the runtime and exposes HTTP views.
- `mempool_processor` consumes live token state and live token events.

This lets us split the runtimes into separate services later without changing
the conceptual data flow.

## Related Docs

- `feed/README.md`: crate-level confirmed live-feed and token-runtime contract.
- `state/README.md`: shared live-state protocol, snapshots, keys, and store traits.
- `../../eth_token_server/README.md`: token-server API and inspector-facing live state exposure.
- `../../mempool_processor/README.md`: mempool risk processing, token context consumption, and `LiveTxSimulator` use.
