# Token Lab

Agent operating map for repeatable token, pool, launch, market-behavior, and
trading-strategy investigations.

## Purpose

- Turn token/pool behavior into reproducible investigations and reusable analysis.
- Compare range-builder output, chain truth, simulator replay, and trading
  guardrail expectations.
- Promote confirmed patterns into production detectors, frontend summaries,
  strategy analytics, or alpha guardrails.

## Owns

- Investigation folders with the narrative, machine-readable facts, and generated
  artifacts for one concrete investigation.
- Strategy analysis notes and contracts for launch/winner/scam cohort stats.
- Read-only triage and detector prototype tools.
- Chain-truth, parity, and trading comparison scripts.
- Shared catalog of odd behavior patterns.

## Does Not Own

- Production token state logic; fix confirmed state bugs in `eth_token`.
- Simulation internals; fix replay mismatches in `tx_simulator` or
  `tx_processor`.
- Live mempool signal emission; promote detectors into `mempool_processor`.
- Strategy policy; promote guardrails into `alpha`.

## Data Flow

```text
token range/server output + simulator logs
  -> read-only triage ledger
  -> investigation folder with chain truth and parity artifacts
  -> confirmed finding
  -> fix in owner crate or promote detector/guardrail
```

## Folder Structure

| Folder | Purpose |
| --- | --- |
| `investigations/` | Concrete token or pool investigations with narrative, metadata, and artifacts. |
| `network_analytics/` | Token-network, pool-network, and second-order fund-flow case studies. |
| `scam_analytics/` | Scammed-pool labels, active targets, model-row exports, and Risk Atlas read models for risk modeling. |
| `odd_behaviors/` | Shared catalog of behavior patterns that deserve review or detectors. |
| `strategy/` | Launch, winner, scam/risk, and cohort analysis used to design trading strategies. |
| `tools/chain_truth/` | Chain-fact extraction used as the baseline for trust. |
| `tools/parity/` | Simulator-vs-chain replay checks. |
| `tools/trading/` | Buy/sell and route behavior checks for execution viability. |
| `tools/detectors/` | Agent-native prototype detectors over completed range/live data. |

## Where To Look First

| Need | Start here |
| --- | --- |
| Working candidate ledger | `investigations/README.md` |
| One concrete investigation | `investigations/<slug>/README.md` |
| Token graph or fund-flow case study | `network_analytics/<slug>/README.md` |
| Scammed-pool labels, targets, and Risk Atlas | `scam_analytics/README.md` |
| Investigation metadata shape | `investigations/<slug>/investigation.toml` |
| Launch/winner strategy stats | `strategy/README.md` |
| Triage candidate generation | `tools/detectors/README.md`, `tools/detectors/range_triage.py` |
| Receipts/logs/balances/reserves truth | `tools/chain_truth/` |
| Simulator parity checks | `tools/parity/` |
| Observed trading comparison | `tools/trading/` |
| Shared suspicious patterns | `odd_behaviors/README.md` |

## Scam Labeling Notes

Scam labels should describe the execution mechanism, not only the observed
outcome. A reserve collapse after LP is burned or sent to `0xdead` is not
automatically a liquidity removal.

The production labeler is pool-first and position-independent. It is implemented
in `eth_token/src/pools/scam_mechanism.rs` as
`BasePool::inferred_scam_mechanism()` and uses reserve-collapse evidence,
Uniswap V2 `Burn`/`Swap` events, prior sell-failure context, and suspicious
token transfers out of the pair that do not have a matching pool event. Token
and live-feed summaries expose the result as `scam_mechanism` plus
`scam_mechanism_label`, so a scam can be classified even if the strategy never
held or sold a position.

| Label | Internal key | Use when | Do not use for |
| --- | --- | --- | --- |
| Backdoored Pair-Balance Drain | `pair_balance_backdoor_drain` | The token contract lets an actor move tokens out of the pair without a valid allowance, then sell those tokens to drain quote reserves. | Direct LP burns/removals or hidden supply expansion. |
| Direct LP Liquidity Removal | `direct_lp_liquidity_removal` | LP tokens are burned/redeemed through the pair or router and reserves leave through a liquidity-removal path. | Reserve collapse caused by a token swap. |
| Reserve Dump / External Holder Drain | `reserve_dump_drain` | A holder sells a large token balance into the pool and drains quote reserves without unauthorized pair movement. | LP redemption, pair-balance backdoors, or sell-blocked privileged-seller patterns. |
| Privileged Seller Reserve Drain | `privileged_seller_reserve_drain` | Strategy/retail sells fail, usually with `TRANSFER_FROM_FAILED`, while another address can router-sell tokens and drain quote reserves through a normal `Swap`. | Pure market dumps where the strategy can also sell, or direct LP removal. |
| Sell-Blocked Honeypot | `sell_blocked_honeypot` | The strategy buy succeeds, but the strategy sell route or address is blocked by token logic. | Route-support gaps, chunk-size sensitivity, or already-drained pools. |
| Max-Sell-Size / Chunkable Exit Restriction | `chunkable_sell_limit` | The full strategy exit fails, but smaller chunks or a known alternate sell size succeed. | Fully blocked sells or drained pools. |
| Hidden Mint / Supply Expansion | `hidden_mint_supply_expansion` | Supply increases through mint behavior or equivalent accounting expansion and the new tokens drain the pool. | Constant-supply transfer backdoors. |
| Critical LP Control | `critical_lp_control` | A current material LP holder grants approval or control that can later remove liquidity. | Approval rows from owners with no LP balance. |
| Simulator Balance Overlay Gap | `simulator_balance_overlay_gap` | The backtest cannot inject or value the strategy token balance because the token uses an unsupported balance layout. | A scam label; this is a simulator coverage gap until chain behavior is reproduced. |

Current canonical example: VYP
`0x8CEDa8619ad186E7C9bCA77734e48c8f518f5d01` is a
`pair_balance_backdoor_drain`, not a hidden mint or direct LP removal. Its total
supply stayed at `1,000,000,000` VYP, LP was dead-address locked, and the drain
came from `transferFrom(pair, actor, 588075766.703258098 VYP)` succeeding with
zero allowance before the same actor sold those tokens in the same block. See
`investigations/vyp_burned_lp_reserve_drain_25077324/`.

Current run note: in
`live-noncapital-maxhold50-riskbundle-delay1-gas-70k-20260514-codex`, the
sell-failed cases are mostly not V2 `Burn` removals. A first pass over the 56
sell-failed positions found 41 reserve-drain swaps before or at our failed exit,
8 reserve-drain swaps after our failed exit, 1 direct LP removal before exit, 1
chunkable/full-size sell restriction, 2 simulator balance-overlay gaps, 2
unclassified sell blocks, and 1 missing token-server history case. The dominant
definition to verify next is `privileged_seller_reserve_drain`: our strategy
cannot transfer/sell, but a privileged address can later sell into the pair and
remove the WETH through a normal swap.

## Tests And Commands

Run from the ETH repo root:

```bash
token_lab/tools/detectors/range_triage.py \
  --api http://127.0.0.1:8765 \
  --run active \
  --format markdown
```

## Current Hazards

- The first artifact should be a candidate ledger, not a fix.
- Do not write generated token-lab artifacts from normal range builds.
  Investigation tools should write under `investigations/<slug>/artifacts/`.
- Promote an investigation only when it affects trading decisions, suggests a
  pipeline bug, or should become a detector/guardrail.
- A number is trusted only after it matches chain behavior or the mismatch is
  explained and tracked.
