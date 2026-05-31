# tx_executor

Workspace for prepared Ethereum transaction execution.

## Layout

| Path | Crate | Purpose |
| --- | --- | --- |
| `tx_executor/` | `tx_executor` | Core library: validates prepared transactions, reserves nonces, signs, dry-runs or broadcasts, and journals execution events. |
| `tx_executor_service/` | `tx_executor_service` | HTTP service: auth, status routes, ETH policy checks, policy journaling, Postgres spend accounting, and calls into the core crate. |
| `tx_executor_signer/` | `tx_executor_signer` | Host-local Unix-socket signer plus keystore importer. Enforces signer-side allowlists and caps before returning a signed raw transaction. |

This workspace is intentionally nested under `blockchains/eth/tx_executor`.
It is not a member of the root ETH workspace because it owns a separate
operational boundary and release surface.

## Runtime Model

```text
alpha/planner
  -> DirectRawTransactionRequest + simulation reference + audit metadata
  -> tx_executor_service POST /eth/tx/direct-raw or /eth/tx/submit
  -> ETH policy + spend reservation
  -> tx_executor core validation, nonce reservation, signing, dry-run/broadcast
  -> optional tx_executor_signer Unix socket for the final signature
  -> SubmitDirectRawResult / policy journal row
```

The executor starts from prepared calldata. It does not choose routes, quote,
slippage, gas-rank candidates, strategy policy, pools, or token risk. Those
belong upstream in alpha and the ETH analysis crates.

## HTTP Service

Default bind:

```text
127.0.0.1:5006
```

Routes:

```text
GET  /health
GET  /eth/tx/status
GET  /eth/tx/policy/decisions
GET  /eth/tx/policy/decisions/{attempt_id}
POST /eth/tx/direct-raw
POST /eth/tx/submit
```

Auth uses `Authorization: Bearer <token>`. Prefer
`ETH_TX_EXECUTOR_API_TOKEN`; `TX_EXECUTOR_API_TOKEN` and `KARTAL_API_TOKEN` are
accepted only as migration fallbacks.

## Config

Shared local defaults live in:

```text
/home/nima/code/crypto/blockchains/eth/config.env
/home/nima/code/crypto/blockchains/eth/config.toml
```

Primary service env:

```bash
ETH_TX_EXECUTOR_BIND=127.0.0.1:5006
ETH_TX_EXECUTOR_RPC_URL=http://127.0.0.1:8545
ETH_TX_EXECUTOR_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/eth_db
ETH_TX_EXECUTOR_BROADCAST_MODE=dry_run
ETH_TX_EXECUTOR_SIGNER_BACKEND=unix_socket
ETH_TX_EXECUTOR_SIGNER_SOCKET_PATH=/run/eth-tx-executor/signer.sock
ETH_TX_EXECUTOR_SIGNER_ADDRESS=0x...
ETH_TX_EXECUTOR_API_TOKEN=...
```

Primary signer env:

```bash
ETH_TX_SIGNER_ADDRESS=0x...
ETH_TX_SIGNER_SOCKET_PATH=/run/eth-tx-executor/signer.sock
ETH_TX_SIGNER_KEY_BACKEND=keystore
ETH_TX_SIGNER_KEYSTORE_PATH=/path/to/eth-signer-keystore.json
ETH_TX_SIGNER_PASSWORD_FILE=/path/to/eth-signer-password
ETH_TX_SIGNER_ALLOWED_TARGETS=0x...
ETH_TX_SIGNER_ALLOWED_SELECTORS=0x8a62666c,0x5f413d10
```

The signer and service both enforce hard caps. Keep the policy caps in
`ETH_TX_POLICY_*` and `ETH_TX_SIGNER_*` aligned before enabling broadcast.

## Wire Contracts

The direct transaction protocol name is `eth_unsigned_tx`.

Canonical request type:

```text
tx_executor::request::DirectRawTransactionRequest
```

Important request fields:

| Field | Meaning |
| --- | --- |
| `attempt_id` | Optional idempotency/audit id. If empty, the executor generates one. |
| `chain_id` | Must match the configured executor chain id. Mainnet is `1`. |
| `from` | Must match the configured signer address. |
| `to` | Target contract/address. |
| `value` | Native ETH value in wei. Defaults to zero. |
| `data` | Prepared calldata. Defaults to `0x`. |
| `gas_limit` | Hard gas limit. |
| `max_fee_per_gas` | EIP-1559 max fee per gas in wei. |
| `max_priority_fee_per_gas` | EIP-1559 priority fee per gas in wei. |
| `nonce` | Optional caller nonce. Prefer `null` so the executor reserves it. |
| `bribe` | Public priority-fee override metadata. |
| `simulation` | Pre-submit simulation reference. Required by the default ETH policy. |
| `metadata` | Caller audit trail and policy evidence. Preserved as JSON. |

Canonical response type:

```text
tx_executor::types::SubmitDirectRawResult
```

Response statuses are `received`, `rejected`, `signed`, `dry_run`,
`broadcast`, and `broadcast_error`.

The Unix-socket signer protocol schema is `eth_tx_signer_v1`, maintained in
`tx_executor::signer::wire`.

## Commands

Run from `/home/nima/code/crypto/blockchains/eth/tx_executor`:

```bash
cargo test
cargo run -p tx_executor --example submit_direct_raw -- --dry-run
cargo run -p tx_executor_service --bin run_eth_tx_executor
cargo run -p tx_executor_signer --bin run_eth_tx_signer
cargo run -p tx_executor_signer --bin import_eth_tx_signer_keystore
```

Build the deployable service binaries:

```bash
cargo build --release -p tx_executor_service -p tx_executor_signer
```

Equivalent absolute-manifest form:

```bash
cargo test --manifest-path /home/nima/code/crypto/blockchains/eth/tx_executor/Cargo.toml
cargo run --manifest-path /home/nima/code/crypto/blockchains/eth/tx_executor/Cargo.toml -p tx_executor_service --bin run_eth_tx_executor
```

## Development Rules

- Keep route construction, quote selection, strategy policy, and gas-rank
  decisions outside this workspace.
- Add new executor behavior through typed request/response fields or a new wire
  protocol version; put caller evidence in `metadata`.
- Use `nonce = null` for normal live flow; manual nonces are recovery tooling.
- Keep live broadcast behind explicit `ETH_TX_EXECUTOR_BROADCAST_MODE=broadcast`
  plus signer key availability.
- Treat `broadcast` as RPC acceptance, not settlement. Receipt watching and
  position reconciliation stay upstream.

## Where To Look First

| Need | Start here |
| --- | --- |
| Core public API | `tx_executor/src/lib.rs` |
| Core submit flow | `tx_executor/src/executor.rs`, `tx_executor/src/service.rs` |
| Request/response types | `tx_executor/src/request/`, `tx_executor/src/types.rs` |
| Validation | `tx_executor/src/validation/` |
| Nonce/signing/broadcast | `tx_executor/src/nonce.rs`, `tx_executor/src/signer/`, `tx_executor/src/broadcast.rs` |
| HTTP routes | `tx_executor_service/src/server.rs` |
| ETH policy | `tx_executor_service/src/policy.rs`, `tx_executor_service/src/repository.rs` |
| Signer daemon | `tx_executor_signer/src/server.rs`, `tx_executor_signer/src/policy.rs` |
| Keystore importer | `tx_executor_signer/src/bin/import_eth_tx_signer_keystore.rs` |
