# Bogaz

`bogaz.md` is the ETH bottleneck ledger. Keep only active bottlenecks here:
what is limiting the pipeline, the evidence, the owning subsystem, and the next
action. Completed work belongs in focused docs or commit history.


## Bogaz Destination Goal

Deploy a real ETH strategy whose policy has been validated by reproducible
recent backtests, live shadow/live-backtest evidence, and audited trade-level
decisions.

This is the Bogaz roadmap target, not the current Codex execution target.

The non-capital path is not the deployment target. It is the calibration layer:
use it to estimate policy behavior against current live data, compare it with
historical backtests, and catch signal/simulation drift before real execution.

Finished means:

- A production strategy policy is selected from the current strategy set.
- The selected policy is backtested over the exact last two-week block window.
- The same policy has live shadow/live-backtest evidence against the current
  chain-server and mempool pipeline.
- Top 10 and worst 10 positions are reviewed with entry reason, skip reason,
  exit reason, gas/slippage assumptions, and PnL snapshot.
- The decision ledger can explain every buy, skip, retry, exit, and failed
  simulation needed for frontend review.
- Execution gates are explicit before real orders are sent: max exposure,
  sizing, retry cadence, exit restrictions, stale-data thresholds, simulation
  freshness, and kill switch behavior.


## Current Limiting Factor

As of 2026-05-21, the live-capital bottleneck is hardcoded runtime control, not
basic transaction submission. Kartal and `tx_executor` can accept, validate,
sign, journal, dry-run, and publicly broadcast a prepared direct transaction.
`eth_alpha_live_trader` can instantiate the Kartal real adapter, target the
deployed Uniswap V2 trading vault, run exact deployed-vault simulations, use the
chain-server gas-rank endpoint, submit through Kartal, and reconcile mined V2
vault buy/sell receipts.

The practical consequence is:

- The one-pool `alpha11-live-univ2-lp30-pool-update-block-hold3-validation`
  public validation has mined buy and sell receipt evidence.
- The main `alpha11-live-univ2-lp30-pool-update-block-hold15` strategy remains
  blocked from broadcast until the hardcoded controls below are reviewed.
- Fixed `40/50 gwei` live gas is removed from the production path, but fee caps,
  gas-rank lookback, profile ladders, simulated-gas buffer, value caps, daily
  caps, and duplicated signer/vault addresses still need explicit ownership.
- Kartal, the signer, Alpha11 strategy specs, `config.env`, and Asena must agree
  before another public run; conflicting caps or duplicated addresses are now the
  highest-risk failure mode.

## Live Tx Submission Audit 2026-05-19

Current intended path for real live execution:

```text
eth_chain_server live pools + mempool signal rows
  -> eth_alpha_trader
  -> LiveSnipeAllStrategy / strategy suite
  -> StrategyDecision::SubmitOrder
  -> OrderIntent
  -> AlphaEngine risk policy + order recording
  -> real TxExecutorAdapter
  -> LiveTradingPlannerBridge
  -> alpha/live/trading LivePrioritySellPlanner + tx_prep
  -> LiveTraderTxSignal / LiveDirectRawTransactionRequest
  -> Kartal POST /eth/tx/direct-raw
  -> tx_executor validate -> reserve nonce -> sign -> dry-run/public broadcast
  -> ExecutionReport back into alpha store
  -> receipt tracker later confirms/rejects the on-chain fill
```

Current live-backtest/shadow path:

```text
eth_alpha_trader
  -> LiveSnipeAllStrategy / strategy suite
  -> StrategyDecision::SubmitOrder
  -> OrderIntent
  -> AlphaEngine risk policy + order recording
  -> LiveChainSimExecutionAdapter
  -> tx_simulator chain-state buy/sell simulation
  -> ExecutionReport with no real tx hash
  -> alpha store
```

The codebase now has the live path connected, but promotion to broader live
capital is blocked by hardcoded controls and operator policy review:

| Layer | Current state | Deployment gap |
| --- | --- | --- |
| Strategy decision | `LiveSnipeAllStrategy` / Alpha11 emits buy/sell `OrderIntent`s for entries, LP approval, liquidity removal, tax/scam, and max-hold when enabled. | Strategy constants and filters are compiled into Alpha11/baseline configs and must be reviewed as part of the hardcoded register. |
| Engine | `AlphaEngine` records decisions and can route approved `OrderIntent`s to either chain-sim or Kartal real execution. | Broader broadcast is blocked by policy review, not by missing adapter plumbing. |
| Real adapter | `TxExecutorAdapter` and `LiveTradingPlannerBridge` are instantiated by `eth_alpha_live_trader` for Kartal real execution. | The current hard blocker is making every real-capital knob explicit and config-owned. |
| Live tx prep | `alpha/live/trading::tx_prep` enforces route validation, exact pre-submit simulation, value-capped bribe policy, gas-rank candidate selection, and protocol metadata. | Fee caps, profile ladders, gas buffer, and simulation freshness are still hardcoded defaults. |
| Route/calldata | The real Alpha11 path uses the deployed Uniswap V2 trading vault for buy and emergency sell calldata. | Actual transaction gas limits remain in code and need benchmark/update ownership; route gas-used estimates now require exact simulation. |
| Gas rank | The real planner calls the chain-server gas-rank endpoint and rejects unranked fixed gas in production. | Lookback and profile choice are hardcoded and must be reviewed against current mined fee distributions. |
| Kartal | `/eth/tx/direct-raw` is mounted and policy-gated; status exposes signer, mode, allowlists, caps, and daily spend. | Kartal `config.toml`, `.env`, signer env, Alpha, and UI currently have duplicated/conflicting values. |
| tx_executor | Direct raw validation, caps, nonce reservation, local signing, journal, dry-run, public mempool broadcast, and policy journal exist. | No private relay/builder route yet; public mempool remains acceptable only for controlled validation until reviewed. |

## Bribe Selection Discussion

For `eth_direct_raw_v1`, the bribe is the EIP-1559 priority fee selected before
Kartal submission. We should not ask Kartal or `tx_executor` to discover the
bribe. Alpha must choose it from current strategy context, recent block-rank
evidence, and protected trade value.

The live planner should use `eth_block_tx_rank` to generate a small set of
ranked candidates from recent mined blocks:

```text
candidate label
priority_fee_gwei
max_fee_per_gas_gwei
expected rank position
expected gas before us
sample/source window
```

Then `alpha/live/trading::tx_prep` applies the economic cap:

```text
avoidable_loss = simulated_recovery_if_fast
               - expected_late_recovery_if_slow
               - safety_buffer

max_priority_spend = avoidable_loss - estimated_base_fee_cost
max_priority_fee_gwei = max_priority_spend / estimated_gas_used
```

Only candidates inside both the priority-spend cap and total-fee cap are
eligible. Public mempool routing should reject if no ranked candidate fits. That
is safer than broadcasting a weak transaction that advertises our exit without a
credible chance of landing early. The planner must not synthesize an unranked
value-cap candidate when rank evidence is missing or too expensive.

Open tuning questions before live capital:

- Which recent-block window should rank against: last block only, rolling N
  blocks, or signal-specific windows around prior scam transactions?
- Which rank band is required by signal type: mempool LP approval likely needs a
  top-of-block band, while mined LP approval may accept a lower band unless
  removal probability is high.
- How stale can gas-rank evidence be during a fast exit path before the planner
  rejects and avoids sending a bad request?
- Should candidate labels and rejected alternatives be persisted with the order
  even when `tx_prep` rejects, so later analysis can tune the cap?

## Live Trading Hardcoded Value Register 2026-05-21

Purpose: this is the review queue before relaxing dry-run or running more public
validation. Every value below is either a money-moving production default, a
runtime safety limit, or a duplicated deployment fact that can cause the live
trader, Kartal, Asena, and the signer to disagree.

Review status key:

- `needs decision`: do not rely on this value for more live capital until we
  explicitly accept it, move it to config, or delete it.
- `acceptable constant`: protocol/unit/ABI constant; keep documented but it is
  not an operator tuning knob.
- `fixture only`: test/calibration default; safe only if it cannot reach
  production execution.

| ID | Area | Current hardcoded value | Why it matters | Review status |
| --- | --- | --- | --- | --- |
| LT-01 | `alpha/engine/src/live_trader/mod.rs` | Default Kartal URL `http://127.0.0.1:5004` | Live trader can silently target a local Kartal instance. We should decide whether this must come from `config.env`/systemd only. | needs decision |
| LT-02 | `alpha/engine/src/live_trader/mod.rs` | Default Kartal token env `ETH_TX_EXECUTOR_API_TOKEN`; no fallback token env is allowed | Auth source is now unambiguous. The live trader reads exactly the configured env var and fails closed when it is missing or empty. | needs decision |
| LT-03 | `alpha/engine/src/live_trader/mod.rs`, `config.env`, Asena trade config | Live `from` / ops address `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27` | This is a real signer address. Duplicating it across Rust, config, Kartal, signer, and UI creates drift risk. | needs decision |
| LT-04 | `alpha/engine/src/live_trader/mod.rs`, `config.env`, Asena trade config | V2 vault `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597` | This is the real execution target. Any wrong duplicate can route calldata/policy to the wrong contract. | needs decision |
| LT-05 | `alpha/engine/src/live_trader/mod.rs` | Live-real validation bankroll ceiling `0.225 ETH` | This is an exposure guard. Decide whether this remains a compile-time validation cap or moves to strategy/deploy config. | needs decision |
| LT-06 | `alpha/engine/src/live_trader/cli.rs` | Poll interval `2000ms`, mempool lookback `14d`, signal limit `200` | Poll cadence and signal window affect latency, duplicate handling, and DB/API load. | needs decision |
| LT-07 | `alpha/engine/src/live_trader/real_execution.rs` | Validation buy size `0.01 ETH` | This is real capital per entry for the validation strategy. It must match Alpha11 and Kartal value caps. | needs decision |
| LT-08 | `alpha/engine/src/live_trader/real_execution.rs`, `alpha/live/trading/src/planner/config.rs` | Production live-real planner requires `eth_chain_server_gas_rank`, max priority `3.5 gwei`, entry gas fee cap `0.0012 ETH`, exit gas fee cap `0.002 ETH`, safety buffer `0.001 ETH` | These are now the first-line protection against repeating the 40 gwei validation spend. They still need one dry-run metadata check before public capital. | partially accepted |
| LT-09 | `alpha/engine/src/live_trader/receipt_reconciliation.rs` | Confirmation depths: accepted `1`, recheck `3` | This controls when mined txs are treated as accepted and when we recheck. Reorg tolerance and stuck/replaced behavior need explicit policy. | needs decision |
| LT-10 | `alpha/live/trading/src/planner/route_builder.rs` | Direct V2 route gas limit `500000`; route estimate is absent until exact simulation fills it | Direct route is not the Alpha11 vault path, but the actual gas limit still affects any future direct route. The old `180000` estimated-gas fallback is removed. | needs decision |
| LT-11 | `alpha/live/trading/src/planner/route_builder.rs`, `tx_simulator::tx_builders` | V2 vault buy/sell gas limits `300000`; route estimate is absent until exact simulation fills it | The old `155000` buy and `130000` sell estimated-gas fallbacks are removed. The remaining question is whether `300000` is the right actual tx gas limit. | needs decision |
| LT-12 | `alpha/live/trading/src/tx_prep/route.rs` | Simulated gas buffer `5000 bps` = 50% | This changes fee cap checks and worst-case cost. It is conservative for readiness because it blocks overpaying sooner, but it should be calibrated against more mined vault receipts. | needs decision |
| LT-13 | `alpha/live/trading/src/planner/gas_rank.rs` | Chain-server gas-rank lookback default `100`, clamped to `1..100` | This now replaces fixed `40 gwei`. We must decide whether `100` blocks is right for fast launch/scam exits. | needs decision |
| LT-14 | `alpha/live/trading/src/tx_prep/strategy_gas_policy.rs` | Entry buy ladder `p50 -> normal`; normal strategy exits `p50 -> normal`; LP/risk race exits `p95 -> p90 -> p75 -> p50 -> normal`; mined approval races `p90 -> p75 -> p50 -> normal`; buy-confirm-block sells `p90 -> p75 -> p50` | This decides which mined gas percentile candidate we try first. Production caps still apply, so high-percentile candidates above `3.5 gwei` are rejected/fallbacked instead of used blindly. | partially accepted |
| LT-15 | `alpha/live/trading/src/planner/simulation.rs` | Max simulation state lag `2` blocks | Kartal also enforces simulation freshness. This must match the chain-server/signer policy or we can accept evidence Kartal rejects. | needs decision |
| LT-16 | `alpha/live/trading/src/planner/min_output.rs`, `baseline/snipe_all/config.rs` | Default slippage `500 bps` and production min-output rejects `>=10000 bps` | Min-output protects against bad fills. The Alpha11 value should be confirmed against live volatility and vault overhead. | needs decision |
| LT-17 | `alpha/live/trading/src/lp_approval_exit.rs`, `shared_rules/lp_approval/mod.rs` | LP approval threshold `30%`, strict `approved_pct > 30` | Equal 30% is allowed. This should remain explicit in strategy docs/UI and not surprise us during validation. | needs decision |
| LT-18 | `alpha/strategies/src/alpha11/config.rs`, `alpha11/live/specs.rs` | Alpha11: buy `0.01 ETH`, initial bankroll `0.225 ETH`, validation bankroll `0.01 ETH`, validation pools `1`, price/initial cap `1.5`, liquidity floors `0.5 ETH`/`1000`, holds `12/15/20`, validation hold `3`, min sell reserve `0` | These define the strategy. They are acceptable as named strategy constants only if the frontend/readme/backtest/live runner all display the same values. | needs decision |
| LT-19 | `alpha/strategies/src/baseline/snipe_all/config.rs` | Baseline defaults: buy `0.01 ETH`, sell fraction `1.0`, min reserve `0.5 ETH`, stable reserve `1000`, min sell reserve `0.01`, denoms `ETH/WETH/USDC/USDT/DAI`, slippage `500 bps`, deadline `30s` | Alpha11 overrides some of these, but any future strategy using baseline directly inherits them. | needs decision |
| LT-20 | `shared_rules/entry/eligibility/mod.rs`, `route_builder.rs`, `config.env` | WETH address `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`; ETH/WETH-only execution route | WETH is a chain constant, but the route restriction is an execution filter and must be reflected in strategy names/docs when active. | acceptable constant for WETH; needs decision for route filter |
| LT-21 | `receipt_reconciliation.rs`, `planner/simulation.rs` | Vault ABI event signatures `BoughtV2(address,uint256,uint256,uint256)` and `EmergencySoldV2(address,uint256,uint256,uint256)` | ABI constants are okay, but changing the deployed vault ABI without updating these would break confirmation. | acceptable constant |
| LT-22 | `alpha/live/trading/src/kartal/client.rs`, `kartal_executor.rs`, `token_server.rs` | HTTP paths: `/eth/tx/status`, `/eth/tx/direct-raw`, `/eth/tx/policy/decisions`, `/eth/tokens/api/live/*`, gas-rank endpoint `/api/v1/eth/alpha/gas-rank/estimate` | Endpoint paths are stable contracts, but they need integration tests so route renames fail loudly. | needs decision |
| LT-23 | `alpha/live/trading/src/calibration/*` | Calibration fixtures include fixed block `25128246`, synthetic vault `0x...02`, `40/50 gwei`, `150000` gas, `0xabc` hash | Fixture values are okay only if isolated from production. They must not be reused as live defaults. | fixture only |
| K-01 | `/home/nima/code/crypto/kartal/src/eth_tx/config.rs` | Executor defaults: RPC `127.0.0.1:8545`, chain id `1`, broadcast `dry_run`, min priority `1 gwei`, max priority `500 gwei`, max fee `1000 gwei` | Kartal is the final spend gate. These should be explicit in deployed config and visible in Asena before broadcast. | needs decision |
| K-02 | Kartal `eth_tx_policy` config/env | Policy caps currently differ by source: repo `config.toml` has daily `1 ETH`; `.env` showed daily `0.06 ETH`; previous status had `0.05 ETH` | Conflicting daily caps make spend accounting confusing. Pick one source of truth before another public tx. | needs decision |
| K-03 | Kartal `eth_tx_policy` and signer policy | Allowed from `0x2348...`, target `0x2847...`, selectors `0x8a62666c`, `0x5f413d10`, value cap `0.01 ETH`, tx cap `0.03 ETH`, gas limit `500000`, fee caps `1000/500 gwei`, sim age `2` | These are the real last line of defense. They must match Alpha11 buy size, vault selectors, and gas-rank policy. | needs decision |
| K-04 | `/home/nima/code/crypto/kartal/src/eth_signer/config.rs` and importer | Signer socket `/run/kartal/eth-signer.sock`, socket mode `0660`, chain id `1`, keystore default backend, importer expected address `0x2348...`, signer tx cap defaults | Signer policy should be at least as strict as Kartal policy. Importer-generated env has its own defaults that can drift. | needs decision |
| UI-01 | `interface/asena/eth/trade/routes/config.py` and docs | Asena duplicates Kartal URL `127.0.0.1:5004`, token server `127.0.0.1:8765`, RPC `127.0.0.1:8545`, signer/vault addresses, run id, simulator token, min ETH out `1`, deadline `1800000000` | The UI is our operator surface. It should read these from the same config/status endpoints when possible, not maintain hidden copies. | needs decision |

Immediate review order:

1. Fee and spend caps: LT-08, LT-12, LT-13, LT-14, K-01, K-02, K-03.
2. Real addresses and deployment targets: LT-03, LT-04, K-03, K-04, UI-01.
3. Strategy exposure and entry filters: LT-05, LT-07, LT-16, LT-17, LT-18,
   LT-19, LT-20.
4. Confirmation/simulation safety: LT-09, LT-15, LT-21, LT-22.
5. Fixture isolation: LT-23.

## Immediate Next Actions

1. Run a Kartal dry-run through the production gas-rank path and confirm the
   request metadata shows `source=eth_chain_server_gas_rank`, a capped profile,
   and priority below `3.5 gwei`.
2. Reconcile Kartal `config.toml`, Kartal `.env`, signer env, Alpha `config.env`,
   and Asena UI so value caps, daily caps, signer address, vault address, and
   selectors have one source of truth.
3. Move real-capital addresses and operator caps out of compiled Rust defaults
   wherever possible; compiled defaults should be safe reject/dry-run defaults.
4. Decide the simulation freshness and confirmation policy: max simulation age,
   accepted depth, recheck depth, stuck tx timeout, replacement behavior, and
   reorg handling.
5. Decide whether public mempool is acceptable beyond the one-pool validation
   trade; otherwise define a new private relay/builder protocol before main
   hold15 broadcast.
6. After each decision, update Alpha11 README, Asena, Kartal config, and this
   register so the operator surface matches the code path.

## Limiting Factors Of Each Module

| Order | Issue | Owner | Latest Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Real planner is dry-run only** | `alpha/engine`, `alpha/live/trading` | `eth_alpha_live_trader` instantiates `TxExecutorAdapter`, targets the deployed V2 vault, and requires Kartal `dry_run`. Entry-enabled runs require `--entry-bankroll-eth <= 0.225`. | Run the bankroll-limited hold15 validation dry run and keep public broadcast disabled until evidence is reviewed. |
| 2 | **Gas-rank spend policy needs final dry-run evidence** | `alpha/engine`, `eth_chain_server`, `alpha/live/trading` | The real planner now rejects fixed/test gas sources in production sell prep, caps live-real priority at `3.5 gwei`, and uses `p50 -> normal` for entries and normal exits. Kartal/signer caps are still looser than Alpha. | Run the production gas-rank dry run, then tighten K-01/K-03 if the metadata matches the new policy. |
| 3 | **Receipt/fill reconciliation needs operational evidence** | `alpha/engine`, `tx_executor`, `kartal` | A receipt reconciliation worker exists and confirms V2 vault `BoughtV2`/`EmergencySoldV2` events, but it still needs live run evidence, timeout handling, and finality policy. | Run the capped dry run with receipt monitoring enabled, then add stuck/replaced/reorg handling before broadcast. |
| 4 | **Private relay/builder execution is not implemented** | `tx_executor`, `kartal` | `BroadcastMode` supports `dry_run` and `public_mempool`; direct coinbase/private bundle bribes are explicitly outside `eth_direct_raw_v1`. | Decide if public mempool is enough for first live test; otherwise add a new protocol/version for relay or bundle submission. |
| 5 | **Config source of truth is split** | `alpha/engine`, `kartal`, `interface/asena` | Signer/from/vault addresses, fee caps, value caps, daily caps, and simulator defaults are duplicated across Rust constants, `config.env`, Kartal config/env, and Asena UI config. | Pick source-of-truth ownership for each live trading value and remove compiled/operator-facing duplicates where possible. |
| 6 | **Direct EOA allowance policy is unresolved** | `alpha/live/trading`, `tx_simulator::tx_builders`, `solidity/baygus-executor` | Mode A now has vault emergency-sell calldata and internal approve+sell semantics. Direct EOA sells still need a live allowance reader or explicit pre-approval deployment policy. | Prefer Mode A for scam exits; only enable direct EOA sells after documenting pre-approval, permit/multicall, or two-transaction approval behavior. |
| 7 | **Live strategy evidence still needs real-planner shadowing** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Chain-sim live-backtest evidence exists, but it does not include Kartal-shaped tx metadata, gas-rank rejects, or signer/RPC failures. | Run the real planner with Kartal `dry_run` and compare every planned priority exit against live chain-sim outcomes. |
| 8 | **Mempool signal latency attribution is incomplete** | `mempool_processor`, `eth_chain_server`, `alpha/engine` | Latest DB evidence shows `live_trading.signal_events` is usually written within sub-second latency, while the running trader can process some signals 7-70 seconds later. The committed receive-timing fields were not visible from the running chain-server/API yet, so the current process was not on the latest timing code. | Restart chain-server and the live-backtest trader on the latest commit, let them run, then compare `detection_timestamp`, `signal_events.created_at`, API `signal_created_at`, trader observation `first_seen_at`, risk event time, and report completion before changing queue or trader architecture. |

## Mempool Signal Timing Bottleneck 2026-05-22

Current conceptual finding: signal production and signal consumption must be
measured separately before changing more code.

Observed latest state before restarting onto commit `1d9e16e3`:

- `mempool_processor` persisted recent signals quickly. Examples from
  `live_trading.signal_events`: detect-to-store ranged from about `6 ms` to
  `962 ms` for the latest sampled rows.
- The running chain-server/API at `40019` still returned the older signal shape;
  it did not expose `signal_source`, `signal_created_at`, or
  `mempool_first_seen_*`, so the process had not picked up the committed timing
  fields yet.
- The running trader had `poll_interval_ms=2000`, but some signals were still
  processed much later than polling alone explains. Example store-to-risk
  delays included about `7.9s`, `9.9s`, `33.9s`, `56.4s`, and `68.6s`.
- The worst sampled behavior looked like sequential trader-side blocking:
  while one signal/engine call generated reports, later signals waited behind
  it. That points first at trader event-loop/backpressure, not at mempool signal
  creation.

What needs to be fixed or proven, in order:

1. Run the latest committed code and prove the process is current. The signal
   API must expose `signal_created_at`, `signal_source`, and optional
   `mempool_first_seen_*`; strategy observations must show an initial
   `received` phase before engine handling.
2. Let the latest code run long enough to capture fresh live signals, then build
   a timing table with these stages: mempool detect, signal stored, API visible,
   trader received, risk event recorded, reports/decision finished.
3. If store-to-receive is high, inspect token-server/API polling, query limits,
   and whether the trader is reading an old process or stale endpoint.
4. If receive-to-decision is high, simplify the trader critical path: risk
   signals should not wait behind expensive chain-sim market/report work when a
   fast exit decision is needed.
5. Only after the latest timing table is clear, revisit the mempool simulation
   queue priority/drop behavior. There is a suspected priority-drop inversion,
   but it should not be patched blindly while the live evidence points at
   trader-side blocking.

Keep this bottleneck simple: the desired output is an explainable timing ledger,
not more queue layers. Every proposed fix should reduce one measured interval in
the timing table above.
