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

As of 2026-05-24, the live-capital bottleneck is production-readiness evidence
and policy/config ownership, not real transaction submission architecture. The
implemented real flow is documented in `alpha/live/trading/README.md` and
`alpha/engine/src/live_trader/real_execution/README.md`:

```text
Alpha strategy/engine
  -> planner
  -> exact deployed V2 vault simulation
  -> gas-rank policy
  -> Kartal direct-raw request
  -> tx_executor
  -> receipt reconciliation
```

The practical consequence is:

- The pipeline for tracking live real trades is implemented: Kartal direct-raw
  submission, receipt reconciliation, vault event parsing, live trade pages, and
  the basic receipt lifecycle are in place.
- The explicit Alpha11 hold16 validation service can public-broadcast through
  Kartal only with `--allow-public-mempool-live-validation`.
- Broader hold15 production remains blocked by public-mempool/private-relay
  policy and operator visibility, not by missing real-trade tracking or missing
  gas outcome fields.
- The current real-validation bankroll limit is `0.555 ETH`, and the live
  gas-rank policy is configured/documented. The remaining parameter task is
  operator visibility: expose the live trading parameters in one review page
  instead of keeping a long bottleneck-register table here.

## Alpha Structure State 2026-05-24

First cleanup pass is complete and was intentionally behavior-preserving:

- `alpha/engine/src/live_trader/mod.rs` was reduced from `1489` lines to `993`
  by extracting bankroll restoration, config resolution, entrypoints, poll-error
  handling, restored runtime state, risk annotation, run metadata, and strategy
  construction.
- `alpha/engine/src/execution/real/mod.rs` now keeps production adapter code in
  the module and moved tests to `execution/real/tests.rs`.
- `alpha/engine/src/live_trader/real_execution.rs` is now folder-shaped:
  `live_trader/real_execution/mod.rs`, `tests.rs`, and `README.md`.
- `live_trader/`, `live_trader/backtest/`, and
  `live_trader/real_execution/` each have a single README explaining the folder
  boundary.
- Stale empty alpha folders were removed.
- No execution semantics changed. Core buy/sell EVM simulation remains in
  `tx_processor/src/trade_simulation`; Alpha still owns strategy/runtime
  orchestration, valuation snapshots, execution reports, and persistence.

Verification run after the split:

```bash
cargo check -p eth_alpha_engine
cargo check -p eth_alpha_engine --bins
cargo check -p eth_alpha_backtest
cargo test -p eth_alpha_engine gate3 --lib
cargo test -p eth_alpha_engine execution::real --lib
cargo test -p eth_alpha_engine live_trader::real_execution --lib
```

Remaining structure bottlenecks:

| Area | Current state | Next action |
| --- | --- | --- |
| Live runner crate boundary | Live backtest and real-live service wiring still live under `eth_alpha_engine::live_trader`. | Create `alpha/live/runner` when we are ready to change package ownership of the live binaries. |
| Live chain-sim gas policy | `live_trader/backtest/chain_sim_gas_policy.rs` is still `943` lines. | Split into policy classification, metadata extraction, and adapter wrapper. |
| Real execution runtime wiring | `live_trader/real_execution/mod.rs` is still `889` lines. | Split resolver, planner, preflight, gas-selection, and adapter-builder modules. |
| Simulated execution adapters | `execution/simulated/mod.rs` is still `881` lines. | Split historical adapter, live adapter, swap execution wrapper, params, and report helpers. |
| Receipt reconciliation | `live_trader/receipt_reconciliation.rs` is still `873` lines. | Split receipt provider, vault event decoder, evidence builder, and batch reconciler. |
| Engine tests | `engine/src/tests.rs` is still `1344` lines. | Split by runtime, valuation, lifecycle, and snapshot invariants. |

## Tail-Entry Production Parity Blockers 2026-05-24

The current tail-entry hardening is useful, but it does not yet prove final
production parity for same-block entry after a trading-enabled mempool
dependency. These are active blockers before trusting tail-entry with live
capital:

| Order | Blocker | Required evidence |
| --- | --- | --- |
| 1 | Non-vacuous coverage checks | Strategy validation must count `trading_enabled` signals, signals with `mempool_entry_evidence`, successful exact-vault eligible signals, tail-entry intents, and tail-entry submitted/confirmed/deferred/failed/cancelled outcomes. Alpha11 tail-entry validation is blocked when this path is not actually exercised. |
| 2 | Config-bound exact-vault evidence | `mempool_entry_evidence` must prove the configured production vault address, chain id, buy amount, and route version, not just a generic exact-vault-shaped route. |
| 3 | Real receipt ordering preservation | Real order journal and receipt reconciliation must preserve `tail_after_tx_hash`, dependency priority fee, dependency gas price, selected priority fee, and ordering intent in final mined evidence. |
| 4 | Dependency-relative tail gas policy | `tail_entry_buy` gas selection must compare our selected fee against the enabling transaction and validate the intended behind-dependency ordering. |
| 5 | Same-block overlay proof | Backtest/live-backtest must either simulate dependency transaction plus exact vault calldata in the same overlay state or explicitly mark the run as post-mine `N+1` only and block same-block readiness. |

## Live Trading Parameter Surface

The old hardcoded-value register has been removed from this bottleneck ledger.
For the current validation path, use the configured `0.555 ETH` bankroll limit
and the live gas-rank policy documented in:

- `alpha/live/trading/README.md`
- `alpha/block_tx_rank/README.md`

Later, build a single operator-facing live trading parameters page in Asena. It
should show the active bankroll, buy size, signer/from address, vault address,
gas-rank policy, gas/fee caps, slippage/min-output policy, simulation freshness,
receipt finality settings, Kartal mode, and signer/Kartal policy caps. That page
is the right place to make duplicated live parameters visible and reviewable;
`bogaz.md` should only track it as a broad product/ops task.

## Limiting Factors Of Each Module

| Order | Issue | Owner | Latest Evidence | Next Action |
| --- | --- | --- | --- | --- |
| 1 | **Live trading parameter page is missing** | `alpha/engine`, `kartal`, `interface/asena` | The validation bankroll is `0.555 ETH` and the gas policy is configured, but operators still need one page showing the active live-capital parameters and where each value came from. | Build an Asena live trading parameters page that pulls active values from Alpha/Kartal/signer status instead of maintaining another hidden duplicate list. |
| 2 | **Direct EOA allowance policy is unresolved** | `alpha/live/trading`, `tx_simulator::tx_builders`, `solidity/baygus-executor` | Mode A now has vault emergency-sell calldata and internal approve+sell semantics. Direct EOA sells still need a live allowance reader or explicit pre-approval deployment policy. | Prefer Mode A for scam exits; only enable direct EOA sells after documenting pre-approval, permit/multicall, or two-transaction approval behavior. |
| 3 | **Live strategy evidence still needs real-planner shadowing** | `alpha/engine`, `alpha/store`, `eth_alpha_trader` | Chain-sim live-backtest evidence exists, but it does not include Kartal-shaped tx metadata, gas-rank rejects, or signer/RPC failures. | Run the real planner with Kartal `dry_run` and compare every planned priority exit against live chain-sim outcomes. |
| 4 | **Mempool signal latency attribution is incomplete** | `mempool_processor`, `eth_chain_server`, `alpha/engine` | Latest DB evidence shows `live_trading.signal_events` is usually written within sub-second latency, while the running trader can process some signals 7-70 seconds later. The committed receive-timing fields were not visible from the running chain-server/API yet, so the current process was not on the latest timing code. | Restart chain-server and the live-backtest trader on the latest commit, let them run, then compare `detection_timestamp`, `signal_events.created_at`, API `signal_created_at`, trader observation `first_seen_at`, risk event time, and report completion before changing queue or trader architecture. |

## Third-Tier Implementation Items

These are planned improvements, but they are not current blockers for the next
validation step or the immediate second-tier cleanup.

| Order | Item | Owner | Current state | Next action |
| --- | --- | --- | --- | --- |
| 1 | **Private relay/builder execution** | `tx_executor`, `kartal` | `BroadcastMode` supports `dry_run` and `public_mempool`; direct coinbase/private bundle bribes are explicitly outside `eth_direct_raw_v1`. | Add a new protocol/version for relay or bundle submission when we are ready to route priority exits outside the public mempool. |

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
