# tx_executor

Agent operating map for prepared Ethereum transaction submission.

## Purpose

- Receive prepared direct transactions from planners/strategies.
- Validate request shape, reserve nonce, enforce fee caps, sign,
  broadcast, and record the result.
- Provide the execution boundary that alpha can eventually call through an
  adapter.

## Current Deployment Status

`tx_executor` is ready to receive a fully prepared direct-raw transaction. It is
not the current bottleneck for strategy-to-Kartal wiring. The missing piece is
upstream: alpha must still build a concrete live planner that turns strategy
`OrderIntent`s into `DirectRawTransactionRequest`s through
`alpha/live/trading::tx_prep`.

What this crate can do now:

- validate chain id, signer address, quantities, calldata, gas caps, and fee
  caps;
- reserve a nonce when `nonce = null`;
- sign through a configured signer backend;
- journal received/signed/dry-run/broadcast events;
- dry-run by default or broadcast to the public mempool when explicitly enabled.

What it does not do:

- choose routes, calldata, slippage, or gas-rank candidates;
- run final route simulations;
- submit private relay/builder bundles;
- watch receipts and translate mined results back into alpha fills.

## Owns

- `DirectRawTransactionRequest`, `SimulationReference`, bribe request metadata,
  and submit results.
- Nonce reservation, signer integration, validation, fee-cap policy, broadcast mode,
  and optional event recording.

## Does Not Own

- Route discovery, quoting, slippage math, pool discovery, or strategy policy.
- Calldata construction except validating a prepared direct transaction.
- Token/mempool analysis, block-rank estimation, or risk decisions.

## Data Flow

```text
planner/strategy adapter
  -> DirectRawTransactionRequest + simulation reference
  -> Kartal order server POST /eth/tx/direct-raw
  -> EthTxExecutor::submit_direct_raw
  -> validate -> reserve nonce -> sign -> broadcast/dry-run
  -> SubmitDirectRawResult / execution record
```

## Kartal Integration

`tx_executor` is a library crate. In live operation it is hosted by Kartal's
existing Polymarket order server, not by a separate ETH-only daemon.

The canonical HTTP/JSON language is `eth_unsigned_tx`, documented below.
`tx_executor` owns the Rust request/response structs; Kartal hosts them over
HTTP; alpha live trading mirrors the JSON client shape and puts
strategy/rank/value-cap evidence in `metadata`.

Current server surface:

```text
http://127.0.0.1:5004/health
http://127.0.0.1:5004/eth/tx/status
http://127.0.0.1:5004/eth/tx/direct-raw
```

The active Compose deployment is `/home/nima/code/crypto/kartal`, service
`order-server`, container `kartal-order-server`. The host port `5005` belongs to
Tengri's live Polymarket market-data service, not Kartal.

Auth is explicit. Use `KARTAL_API_TOKEN` as one shared token for every Kartal
execution route, or set separate scoped tokens:
`POLYMARKET_ORDER_API_TOKEN` for Polymarket routes and
`ETH_TX_EXECUTOR_API_TOKEN` for ETH tx routes. Scoped tokens do not fall back to
each other.

Runtime config lives in Kartal's `[eth_tx_executor]` config section and matching
`ETH_TX_EXECUTOR_*` environment variables. The default broadcast mode is
`dry_run`. The default signer backend is `env` for local development; live mode
should use `ETH_TX_EXECUTOR_SIGNER_BACKEND=unix_socket` with
`ETH_TX_EXECUTOR_SIGNER_SOCKET_PATH` and `ETH_TX_EXECUTOR_SIGNER_ADDRESS`.

When Kartal runs under Compose, it shares the VPN container network namespace.
The Ethereum RPC endpoint must be reachable from that namespace. The default
Compose value is `http://172.18.0.1:8545`; a host `reth` process bound only to
`127.0.0.1:8545` will reject container connections until it is bound or proxied
onto the Docker bridge.

## Unsigned Tx Wire Protocol

Protocol name: `eth_unsigned_tx`.

This is the wire language between alpha live trading, Kartal, and
`tx_executor`.

```text
alpha live trading / tx prep
  -> POST /eth/tx/direct-raw on Kartal
  -> Kartal auth + JSON decode into tx_executor::DirectRawTransactionRequest
  -> tx_executor validate -> reserve nonce -> sign -> dry-run or broadcast
  -> Kartal returns tx_executor::SubmitDirectRawResult
```

Endpoint:

```text
POST /eth/tx/direct-raw
Authorization: Bearer <ETH_TX_EXECUTOR_API_TOKEN or KARTAL_API_TOKEN>
Content-Type: application/json
```

Kartal also exposes:

```text
GET /eth/tx/status
```

### Request

Canonical type:

```text
tx_executor::request::DirectRawTransactionRequest
```

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `attempt_id` | string or null | no | Idempotency/audit id from caller. If empty/missing, executor generates one. |
| `chain_id` | integer | yes | Must match Kartal executor chain id. Mainnet is `1`. |
| `from` | string | yes | Signer address expected by the executor. Must equal Kartal signer. |
| `to` | string | yes | Target contract/address. For swaps, this is normally the router. |
| `value` | decimal or hex quantity string | no | Native ETH value in wei. Defaults to `"0"`. |
| `data` | hex bytes string | no | Prepared calldata. Defaults to `"0x"`. |
| `gas_limit` | decimal or hex quantity string | yes | Hard gas limit for the transaction. |
| `max_fee_per_gas` | decimal or hex quantity string | yes | EIP-1559 max fee per gas in wei. |
| `max_priority_fee_per_gas` | decimal or hex quantity string | yes | EIP-1559 priority fee per gas in wei. |
| `nonce` | decimal or hex quantity string or null | no | Optional caller-provided nonce. Prefer `null` so executor reserves. |
| `bribe` | object or null | no | Priority-fee override metadata, described below. |
| `simulation` | object or null | strongly recommended | Pre-submit simulation reference. |
| `metadata` | object | strongly recommended | Caller audit trail and policy evidence. |

All numeric transaction quantities are strings because they can exceed normal
JSON integer precision. Decimal strings are preferred; hex quantity strings with
`0x` are accepted by `tx_executor`.

### Bribe Object

In v1, `bribe` means the public EIP-1559 priority fee paid by the EOA
transaction. Direct `block.coinbase` transfers and private bundle payments are
not represented by this direct-raw protocol.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `priority_fee_per_gas` | decimal or hex quantity string | yes | Requested priority fee per gas in wei. Executor uses the max of this and `max_priority_fee_per_gas`. |
| `max_fee_per_gas` | decimal or hex quantity string or null | no | Optional max-fee override in wei. |

The executor still enforces its configured hard caps after applying `bribe`.

### Simulation Object

Canonical type:

```text
tx_executor::request::SimulationReference
```

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `block_number` | integer | yes | Block used for the final pre-submit simulation. |
| `block_hash` | string or null | no | Block hash for audit/replay. |
| `state_root` | string or null | no | State root when available. |
| `expected_output_token` | string or null | no | Token expected from the transaction. |
| `expected_output_amount` | string or null | no | Expected output raw amount. |
| `min_output_amount` | string or null | no | Minimum acceptable output raw amount encoded into calldata/slippage. |
| `metadata` | object | no | Extra simulation evidence. |

The executor records this object but does not re-run route/slippage analysis.
If simulation evidence is stale or missing, fix the caller-side planner.

### Metadata Contract

`metadata` is where alpha communicates why the transaction exists. Kartal and
`tx_executor` treat it as audit data and must preserve it.

Required for alpha priority exits:

```json
{
  "wire_protocol": "eth_unsigned_tx",
  "intent_kind": "priority_sell",
  "executor_boundary": "kartal_eth_tx_executor",
  "tx_prep_version": 1,
  "strategy_name": "alpha11-03-live-v2-hold20-retry3-gasguard",
  "strategy_run_id": "alpha11-live-...",
  "trade_id": "trd_...",
  "token_address": "0x...",
  "pool_address": "0xtoken:0xpool",
  "observed_block": 25128246,
  "reason": "exit.mempool_liquidity_removal_signal",
  "budget": {
    "avoidable_loss_eth": "0.0084",
    "max_total_fee_eth": "0.002",
    "predicted_base_fee_gwei": "0.6",
    "estimated_base_fee_cost_eth": "0.00015",
    "max_priority_spend_eth": "0.00185",
    "max_priority_fee_gwei": "3.5",
    "max_fee_per_gas_gwei": "4.1",
    "estimated_gas_used": 250000
  },
  "gas_plan": {
    "label": "balanced",
    "priority_fee_gwei": "2",
    "max_fee_per_gas_gwei": "2.6",
    "rank_position_p50": 25,
    "gas_before_p50": 900000,
    "likely_fits_at_p50": true,
    "source": "eth_chain_server_gas_rank"
  }
}
```

Development rule: new planner facts go into `metadata`; new executor behavior
requires a new protocol version or a typed request field.

### Response

Canonical type:

```text
tx_executor::types::SubmitDirectRawResult
```

| Field | Meaning |
| --- | --- |
| `attempt_id` | Attempt id accepted by executor. |
| `status` | `received`, `rejected`, `signed`, `dry_run`, `broadcast`, or `broadcast_error`. |
| `tx_hash` | Signed/broadcast tx hash when available. |
| `from`, `to`, `nonce`, `gas_limit`, `max_fee_per_gas`, `max_priority_fee_per_gas` | Final transaction envelope values. |
| `error` | Error text for rejected/broadcast-error outcomes. |
| `elapsed_ms` | Executor-side elapsed time. |

Kartal maps validation/config problems to HTTP validation errors, auth failures
to auth errors, and nonce/broadcast/RPC problems to network errors.

## Local Signer Wire Protocol

When `unix_socket` signing is enabled, `tx_executor` talks to the local signer
with newline-delimited JSON over a Unix socket. The schema name is
`kartal_eth_signer_v1`, maintained in `tx_executor::signer::wire`.

Request kinds:

| Kind | Payload |
| --- | --- |
| `status` | Returns signer address, chain id, and readiness. |
| `sign_direct_raw` | Carries `PreparedDirectRawTransaction` after validation and nonce reservation. |
| `sign_flashbots_auth` | Signs the Flashbots relay body hash for `X-Flashbots-Signature`. |

Response kinds:

| Kind | Payload |
| --- | --- |
| `status` | `SignerStatus` |
| `signed` | signer address plus `SignedTransaction` |
| `signed_flashbots_auth` | signer address plus `SignedFlashbotsAuth` |
| `error` | signer-side rejection or signing error text |

The signer must enforce its own allowlist and caps before returning a raw
signed transaction. That gives us a second policy boundary if Kartal is
misconfigured or an authorized caller submits an unexpected transaction.

Flashbots auth signatures are encoded as `r || s || v`, with the recovery byte
normalized to `0` or `1` for relay compatibility.

### Example Priority Sell

```json
{
  "attempt_id": "trd_mpce88f0_plrm_f-priority-exit-25128246",
  "chain_id": 1,
  "from": "0x1111111111111111111111111111111111111111",
  "to": "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
  "value": "0",
  "data": "0x...",
  "gas_limit": "180000",
  "max_fee_per_gas": "50000000000",
  "max_priority_fee_per_gas": "40000000000",
  "nonce": null,
  "bribe": {
    "priority_fee_per_gas": "40000000000",
    "max_fee_per_gas": "50000000000"
  },
  "simulation": {
    "block_number": 25128246,
    "block_hash": "0x...",
    "state_root": null,
    "expected_output_token": "WETH",
    "expected_output_amount": "10000000000000000",
    "min_output_amount": "9000000000000000",
    "metadata": {
      "expected_recovery_eth": "0.01",
      "would_revert": false
    }
  },
  "metadata": {
    "wire_protocol": "eth_unsigned_tx",
    "intent_kind": "priority_sell",
    "executor_boundary": "kartal_eth_tx_executor",
    "tx_prep_version": 1,
    "reason": "exit.mempool_liquidity_removal_signal"
  }
}
```

### Development Rules

- `tx_executor` owns request/response shape and validation semantics.
- Kartal owns HTTP auth, hosting, config loading, and mapping executor errors
  into HTTP errors.
- Alpha owns route choice, calldata construction, slippage, value budget,
  gas-rank selection, and metadata.
- Never add route/slippage/risk policy to `tx_executor`.
- Never let Kartal mutate `metadata`, except to preserve or wrap it in storage.
- Use `nonce = null` for normal live flow; manual nonce is for recovery tooling.
- Private bundle handoff must invalidate the nonce cache after relay handoff or
  error, because those transactions are not public-pending until inclusion.
- Every real-capital request must include simulation evidence and value-cap
  metadata.
- Live broadcast requires explicit Kartal config:
  `ETH_TX_EXECUTOR_BROADCAST_MODE=broadcast` and signer key availability. The
  raw `public_mempool` value remains accepted for direct public submission, but
  `broadcast` is the operator-facing umbrella that can include public mempool
  or Flashbots/private relay routes selected by the request policy.

## Where To Look First

| Need | Start here |
| --- | --- |
| Public API and exports | `src/lib.rs` |
| Submit flow | `src/executor.rs`, `src/service.rs` |
| Request/response types | `src/request.rs`, `src/types.rs` |
| Validation | `src/validation.rs` |
| Nonce/signing/broadcast | `src/nonce.rs`, `src/signer.rs`, `src/broadcast.rs` |
| Standalone example | `examples/submit_direct_raw.rs` |
| Kartal HTTP host | `/home/nima/code/crypto/kartal/src/eth_tx/` |

## Tests And Commands

```bash
cargo run --manifest-path tx_executor/Cargo.toml --example submit_direct_raw -- --dry-run
cargo test --manifest-path tx_executor/Cargo.toml
```

Live broadcast must be explicit:

```bash
cargo run --manifest-path tx_executor/Cargo.toml --example submit_direct_raw -- --broadcast
```

## Current Hazards

- This crate starts from prepared calldata. If route/quote/slippage decisions
  are missing, fix the planner or strategy adapter, not the executor.
- Rough block-position and gas-before estimates belong in
  `alpha/block_tx_rank`, before the final transaction reaches this crate.
- Direct EOA priority fee is the normal validator/builder payment. Explicit
  `block.coinbase` payments require contract calldata and are not direct raw
  mode.
- A submitted payload should include simulator block/hash and expected/min
  output metadata so execution can be traced back to the off-chain decision.
- A `broadcast` result means the raw transaction was accepted by the RPC path,
  not that the trade filled. Alpha still needs a receipt watcher before real
  deployments can be considered settled.
