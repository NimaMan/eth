# ETH Chain Server API Surfaces

`eth_chain_server` has three client audiences. They share the same process for
now, but they should not share the same contract.

## Surfaces

| Surface | Path / Boundary | Transport | Primary Client | Contract |
| --- | --- | --- | --- | --- |
| Frontend/lab | `/api/v1/eth/...` | HTTP JSON + SSE | Asena, browser pages, lab tools | Read-model views and operator actions. Endpoints may return page DTOs and can be stale by a bounded interval during active builds. |
| Trading | in-process events now; `/api/v1/eth/trading/...` as a narrow facade | Rust event/state boundary now, gRPC later if out of process | Live strategy runtime | Only committed-chain state after block apply. No frontend table scans, Risk Atlas rendering, or analytics exports on this path. |
| Agents | `/api/v1/eth/agents/...` | HTTP JSON | Coding/research agents and automation | Stable orientation and tool-friendly status. Agents use versioned HTTP resources, not internal trading state. |

Agents should not use the internal trading boundary directly. The internal
boundary is for low-latency code that runs inside the trading system and may
change with strategy/runtime ownership. Agents need a stable, discoverable,
schema-light API that is safe to call from tools, so they get a third surface:
the agent/automation API.

## Versioned HTTP Entrypoints

The existing `/eth/tokens/api/...` routes remain compatibility routes. New
clients should use `/api/v1/eth/...`.

Core frontend/lab routes:

```text
GET  /api/v1/eth/health
GET  /api/v1/eth/live/status
POST /api/v1/eth/live/start
POST /api/v1/eth/live/stop
GET  /api/v1/eth/live/updates?after_block=<block>
GET  /api/v1/eth/live/tokens
GET  /api/v1/eth/live/tokens/<token>
GET  /api/v1/eth/live/pools?status=<active|scam|eligible|ineligible>
GET  /api/v1/eth/live/surface

GET  /api/v1/eth/ranges
POST /api/v1/eth/ranges
GET  /api/v1/eth/ranges/active
POST /api/v1/eth/ranges/active/stop
GET  /api/v1/eth/ranges/<run>/progress
GET  /api/v1/eth/ranges/<run>/tokens
GET  /api/v1/eth/ranges/<run>/pools
GET  /api/v1/eth/ranges/<run>/surface
GET  /api/v1/eth/ranges/<run>/errors
GET  /api/v1/eth/ranges/<run>/stream
POST /api/v1/eth/ranges/<run>/risk-atlas/export

GET  /api/v1/eth/analytics/risk-atlas
GET  /api/v1/eth/analytics/risk-atlas/runs
GET  /api/v1/eth/analytics/risk-atlas/runs/<atlas_run>
```

Trading-safe HTTP facade:

```text
GET /api/v1/eth/trading/live/status
GET /api/v1/eth/trading/live/updates?after_block=<block>
GET /api/v1/eth/trading/live/processed-blocks
```

This facade exists for smoke tests, external supervision, and a future
out-of-process trading bridge. The trading runtime should still prefer the
in-process committed event/state path while it lives in the same Rust process.

Agent/automation routes:

```text
GET /api/v1/eth/agents/manifest
GET /api/v1/eth/agents/status
```

Agents should start with `manifest`, then use `status` to discover whether a
live run, range run, or Risk Atlas snapshot is available before calling heavier
read endpoints.

## Agent Run-Mode Rules

Agents should choose the Risk Atlas execution mode before starting work:

| Request shape | Agent should use | Reason |
| --- | --- | --- |
| "Generate 100K/500K data", "train/evaluate model", "build scam distributions" | Headless Risk Atlas generation | No frontend state is needed; DB rows are the canonical output. |
| "Run this range and let me inspect tokens in the frontend/token builder" | Server-backed range builder | Asena needs range snapshots and token/pool read models. |
| "Show/redesign/read the Risk Atlas page" | DB-backed read mode | The page should read Risk Atlas DB views, not active range memory. |
| "Make a trading decision/live gate/exit on next block" | Trading committed-state path | Trading must consume state after full block apply, not frontend DTOs. |

For server-backed range builds, the server should update cheap progress every
block but refresh heavy read models on an interval. The first target interval for
large builds is every `1000` blocks plus a final refresh at completion.

For headless generation, `eth_chain_server` should not be required. The Risk
Atlas generator should process blocks, write bounded batches into the DB, and
derive page/model aggregates from those DB rows.

## Live Trading Event Order

The live trading path must stay committed-state only:

```text
new confirmed block
  -> tx_processor builds processed block facts
  -> eth_token applies every tx and post-block pool simulation
  -> eth_chain_server commits token/pool state
  -> BlockApplied / chain-state-applied event is published
  -> trading/risk consumers read committed state
  -> frontend/agent read models refresh after that
```

Frontend and agent endpoints can observe this state, but they are not the source
of trading truth.

## Folder Ownership

Current target ownership:

```text
src/app/              config, logging, dependency construction
src/live/             server-owned live block orchestration
src/ranges/           historical/range run lifecycle and hot apply loop
src/http/             HTTP transport, route mapping, SSE
src/http/routes/      compatibility routes plus v1 route surface
src/read_models/      DTO/read-model construction for HTTP clients
src/stores/           database-backed stores
src/token_analytics/  long-running token analytics jobs
```

Future split when the trading surface needs to run out of process:

```text
src/api/http/         frontend, agents, ops
src/api/trading/      gRPC or typed service contract
src/events/           chain-state-applied event types and fanout
src/services/         live/range/analytics orchestration
```

Do not move code only to satisfy the target shape. Move it when a module starts
mixing transport, read-model materialization, and hot-path state mutation.
