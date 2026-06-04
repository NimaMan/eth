# Live Go-Live Readiness

Status: **blocked**

This is the release gate for relaxing the ETH tx executor from `dry_run` to
`public_mempool` for the Ethereum alpha live strategy that targets the deployed
`UniswapV2TradingVault`.

Passing this document means the live path can submit real mainnet transactions
from the dedicated hot wallet. It does not mean the strategy is profitable. It
means the execution system has explicit controls for the major failure classes
that can lose funds or corrupt position state.

## Current Decision

Do not relax dry-run yet.

The receipt reconciliation worker now exists in alpha, but the real-live path
still has blockers:

- real live entry is enabled only for capped validation runs with a resolved
  strategy bankroll of at most `0.225 ETH`;
- deployed V2 vault buy and sell paths use production exact calldata
  simulation, but still need live dry-run evidence for the target strategy;
- the gas-rank provider is fixed/shadow, not backed by recent block-rank
  evidence;
- stuck, dropped, replaced, and reorged transactions do not yet have a finality
  policy;
- per-strategy and per-token spend caps are not implemented in the ETH tx executor.

## Required Green Gates

Every item in this section must be `ready` before broadcast.

| Gate | Status | Required Evidence |
| --- | --- | --- |
| Strategy scope | blocked | The live strategy name includes every protocol/filter restriction. If using this V2 vault, live broadcast strategy must be explicitly V2-only. |
| Entry route | partial | Vault buy route is implemented, simulated, and wired into real live trading. Need capped live dry-run evidence for `alpha11-live-univ2-lp30-pool-update-block-hold15`. |
| Exit route | partial | Vault emergency-sell route exists with production min-output and exact final simulation. Need live dry-run evidence. |
| Exact pre-submit simulation | partial | Submitted V2 vault buy/sell requests include exact calldata simulation from current chain state and reject stale state. Need run-folder evidence from the target strategy. |
| Min-output/slippage | partial | `min_output_amount` is derived from exact simulation and policy for V2 vault buy/sell. Need live dry-run evidence proving non-zero min-output on real candidates. |
| Gas-rank provider | blocked | Chosen EIP-1559 fees come from recent `eth_block_tx_rank` evidence. Fixed shadow fees are rejected for broadcast. |
| Value cap | blocked | Priority spend and total max-fee spend are capped by protected value, late-recovery value, and safety buffer from final simulation. |
| ETH tx executor policy | partial | Target, selector, `from`, value, gas, fee, simulation freshness, metadata, and daily spend gates exist. Need final deployed config proof. |
| Signer policy | partial | Unix-socket signer exists. Need final deployed config proof matching vault target, selectors, value/gas/fee caps, and expected signer address. |
| Per-strategy/token spend caps | blocked | ETH tx executor enforces caps beyond global daily cap before materially funding the hot wallet. |
| Receipt reconciliation | partial | Alpha worker exists and unit tests pass. Need live integration evidence and finality policy. |
| Stuck/replaced tx handling | blocked | Null receipts after timeout are surfaced, replacements are detected, and operators have a cancel/replace playbook. |
| Reorg/finality policy | blocked | Confirmed reports are emitted only after the configured confirmation depth, or shallow confirmations are explicitly accepted with alerting. |
| Idempotency | partial | Attempt ids and submitted/final reports are keyed by order id. Need duplicate-submit and restart tests against Postgres. |
| Nonce safety | partial | Tx executor reserves nonce. Need restart, concurrent request, and broadcast-error tests with the live nonce store. |
| Kill switch | partial | `ETH_TX_EXECUTOR_DISABLED` exists. Need a drill proving it stops signing/broadcast while services stay observable. |
| RPC isolation | partial | Reth RPC is consumed directly by the ETH tx executor. Need current systemd status and health evidence in the release run folder. |
| Observability | blocked | Trade page shows broadcast mode, signer backend, caps, spend, submitted/confirmed/failed/unresolved receipts, and stuck tx age. |
| Dry-run soak | blocked | Run production-shaped strategy through ETH tx executor dry-run for a fixed window and store policy decisions, requests, and alpha reports. |
| Hot-wallet funding | blocked | Dedicated wallet balance is small enough that max loss is survivable and aligned with ETH tx executor per-day/per-strategy caps. |
| Operator signoff | blocked | Release folder contains config hashes, command transcript, status snapshots, and explicit operator approval. |

## Failure Classes And Controls

### Bad Strategy Signal

Risk:

- strategy buys a protocol/path the vault cannot exit;
- hidden protocol filter causes historical/live mismatch;
- duplicate buy/sell is submitted for the same position;
- strategy acts on stale or incomplete pool/tracker data.

Controls required:

- strategy name must include protocol/filter restrictions;
- one shared lifecycle: intent, submitted, confirmed, failed, cancelled;
- no position may become confirmed without execution evidence;
- live dry-run must be compared against historical/live-backtest for the same
  strategy and block range;
- new entries stay disabled until the V2 buy route is production-ready.

### Bad Calldata

Risk:

- target or selector is wrong;
- calldata sells the wrong token/amount;
- buy sends ETH to the wrong route;
- sell proceeds do not go to treasury;
- approval/router behavior differs from the vault tests.

Controls required:

- alpha builds only deployed-vault calldata for the selected route;
- ETH tx executor allowlists the exact vault target and selectors:
  `0x8a62666c` and `0x5f413d10`;
- signer policy independently allowlists the same target/selectors;
- final simulation is attached to the exact request;
- receipt reconciliation confirms only `BoughtV2` or `EmergencySoldV2` from the
  deployed vault and matching token.

### Bad Simulation Or Quote

Risk:

- simulation is stale;
- final calldata was not the calldata simulated;
- min-output is unsafe;
- token has transfer tax, blacklist, max-tx, or honeypot behavior that changes
  after simulation.

Controls required:

- ETH tx executor rejects missing/stale simulation references;
- alpha rejects broadcast when simulation says revert;
- `min_output_amount` is derived from expected recovery and slippage policy;
- current-state simulations cover representative success and negative cases;
- live dry-run stores the final request and simulation evidence for review.

### Bad Gas Or Bribe

Risk:

- priority fee is too low and exit misses the block;
- priority fee is too high and burns more than protected value;
- base fee/max fee creates unexpected worst-case spend;
- unranked fallback chooses arbitrary gas.

Controls required:

- gas candidate must come from recent block-rank evidence;
- priority spend is capped by final simulated value;
- total max fee is capped by policy;
- ETH tx executor and signer both enforce hard gas and fee caps;
- policy journal records gas-rank label, source window, expected rank, and
  estimated spend.

### Bad Signing Or Key Handling

Risk:

- private key leaks;
- ETH tx executor signs arbitrary target/calldata after compromise;
- wrong signer owns the vault;
- env/config accidentally flips live mode.

Controls required:

- production uses `unix_socket` signer with encrypted keystore;
- order server never receives raw private key;
- signer policy independently enforces target, selector, value, gas, fee, and
  transaction-cost caps;
- hot wallet owns the vault and holds only limited funds;
- dry-run and public-mempool modes are visibly different in status and run
  metadata.

### Bad Broadcast Or Nonce

Risk:

- duplicate transaction for one order;
- nonce collision;
- stuck transaction leaves position `Submitted` forever;
- replacement transaction is not reconciled;
- broadcast error is treated as failed fill.

Controls required:

- attempt id is stable and persisted before submission;
- nonce reservation survives restart and concurrent requests;
- null receipt past timeout is an alert, not a silent state;
- replacement/cancel policy is documented and tested;
- broadcast result means submitted only, never confirmed.

### Bad Receipt Or State Reconciliation

Risk:

- successful receipt with unrelated event is marked confirmed;
- reverted tx is missed;
- receipt from a reorged block is accepted;
- confirmed sell PnL is wrong;
- alpha writes final DB state without matching position.

Controls required:

- receipt worker requires matching vault event and token;
- reverted receipts become failed reports;
- successful receipts without vault evidence remain unresolved;
- finality depth is configured before confirmed reports are treated as settled;
- engine applies final reports through the same lifecycle path as backtest.

### Bad Operations

Risk:

- no one notices rejected policy decisions, stuck txs, or kill-switch state;
- RPC is unavailable or on the wrong chain;
- service restart loses state;
- dashboards show dry-run metrics as real fills.

Controls required:

- trade page exposes broadcast mode, signer backend, execution disabled,
  policy caps, spend used/remaining, and unresolved receipt count;
- policy journal and signer journal are reviewed during dry-run soak;
- Reth/Lighthouse health and chain id are checked before broadcast;
- systemd/docker restart test is run with an active submitted order;
- kill switch drill is recorded.

## First Acceptable Public-Mempool Scope

The first real run must be deliberately tiny:

- dedicated hot wallet only;
- V2-only strategy name;
- either sell-only for existing vault-held positions, or buy route fully green;
- one strategy instance;
- low per-transaction cap;
- low daily cap;
- low per-strategy and per-token caps;
- operator watching the trade page, ETH tx executor status, and logs live;
- immediate rollback path: `ETH_TX_EXECUTOR_DISABLED=true` or return
  `ETH_TX_EXECUTOR_BROADCAST_MODE=dry_run`.

## Evidence To Store In The Release Run Folder

- git revision and dirty-state summary for alpha, ETH tx executor, and
  vault source;
- ETH tx executor `/eth/tx/status` JSON;
- signer `/status` or systemd status;
- policy config hashes;
- dry-run request/response pairs;
- policy journal excerpts for accepted and rejected cases;
- signer journal excerpts;
- exact simulation reports for the final buy/sell calldata;
- gas-rank evidence window;
- receipt reconciliation test report;
- kill-switch drill output;
- final operator signoff.
