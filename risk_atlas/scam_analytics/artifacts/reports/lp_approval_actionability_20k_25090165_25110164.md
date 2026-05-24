# LP Approval Actionability, Last 20K Blocks

Source range run: `run-2`
Risk Atlas run: `risk-atlas-run-2`
Block range: `25,090,165` to `25,110,164`
Generated: `2026-05-16`

## Surface

- Tokens tracked: `1,884`
- Pool rows: `1,899`
- Eligible pools: `1,443` (`75.99%`)
- Ineligible pools: `456` (`24.01%`)
- Scam-labeled pools: `1,280`
- Active observation rows exported: `73,286`

Scam mechanisms:

| Mechanism | Pools | Share |
| --- | ---: | ---: |
| direct LP liquidity removal | 576 | 45.00% |
| pair balance backdoor drain | 461 | 36.02% |
| unknown reserve drain | 219 | 17.11% |
| reserve dump drain | 24 | 1.88% |

Protocol split:

| Protocol | Eligible Pools | Scam Pools |
| --- | ---: | ---: |
| UNISWAP-V2 | 1,105 | 1,039 |
| UNISWAP-V4 | 287 | 238 |
| UNISWAP-V3 | 51 | 3 |

Direct LP removals are almost entirely V2 in this window: `573 / 576` direct LP scam pools are `UNISWAP-V2`.

## Actionability Definition

We use the block-end view:

- LP approval mined in block `B`.
- The approval is visible after block `B`.
- We can submit an exit for block `B + 1`.
- If liquidity removal is also in `B + 1`, the exit is an ordering race. It only works if our sell is ordered before the removal.
- If liquidity removal is in `B + 2` or later, the exit is clean under a next-block execution assumption.

The measured lead is:

`lead_blocks = liquidity_removal_block - last_pre_removal_lp_approval_block`

## Direct LP Warning Window

Eligible direct LP scam pools: `576`.

| Window | Count | Share of eligible direct LP scams | Interpretation |
| --- | ---: | ---: | --- |
| no pre-removal approval | 5 | 0.87% | no approval warning from mined-chain observations |
| one-block window | 58 | 10.07% | B+1 ordering race |
| two-plus-block window | 513 | 89.06% | clean next-block exit window |

Sellability at the approval block:

| Condition | Count |
| --- | ---: |
| pre-removal approval rows | 571 |
| can sell at approval | 567 |
| effective sell at approval | 567 |
| one-block and effective sell | 56 |
| two-plus blocks and effective sell | 511 |

Lead distribution for eligible direct LP scams with pre-removal approval:

| Metric | Blocks |
| --- | ---: |
| min | 1 |
| p25 | 6 |
| median | 68 |
| p75 | 79 |
| p90 | 107 |
| p95 | 426 |
| max | 3,407 |

Within-short-window counts:

| Lead window | Count | Share of pre-approval rows |
| --- | ---: | ---: |
| 1 to 5 blocks | 136 | 23.82% |
| 1 to 10 blocks | 172 | 30.12% |
| greater than 10 blocks | 399 | 69.88% |

## Approval Timing After Trading Enabled

For eligible direct LP scams with a pre-removal approval, the final pre-removal approval usually arrives very early after trading is enabled.

| Approval timing after trading enabled | Count | Share |
| --- | ---: | ---: |
| within 1 block | 291 | 51.50% |
| within 2 blocks | 386 | 68.32% |
| within 3 blocks | 432 | 76.46% |
| within 5 blocks | 464 | 82.12% |
| within 10 blocks | 486 | 86.02% |
| within 50 blocks | 510 | 90.27% |
| within 100 blocks | 518 | 91.68% |

This means LP approval is not only an exit signal. For many pools it is also a launch-time avoid or immediate de-risk signal.

## LP Approval Size

The current exported column `lp_approved_pct_as_of` is router-specific. In this range it reads as zero because the range processor was created without a known-router list, so router approval classification is not reliable.

The generic approval amount in `features.lp_control.lp_max_approval_amount_as_of` is reliable for these rows. Capping it at `100%` of LP total supply:

| Feature | Count |
| --- | ---: |
| max LP approval >= 90% of LP supply | 571 / 571 |
| top LP holder >= 90% of LP supply | 571 / 571 |
| one LP holder | 529 / 571 |
| approval owner is token creator | 566 / 571 |

So the useful feature is generic LP approval size, not router-only approved percentage.

## Scam Timing

Trading-enabled to scam label, by mechanism:

| Mechanism | Count | P25 | Median | P75 | P90 | P95 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| direct LP liquidity removal | 575 | 29 | 74 | 86 | 377 | 893 |
| pair balance backdoor drain | 461 | 20 | 31 | 48 | 85 | 161 |
| unknown reserve drain | 212 | 67 | 88 | 608 | 1,964 | 4,393 |
| reserve dump drain | 19 | 24 | 45 | 133 | 167 | 254 |

Pair-balance backdoor drains are much faster than direct LP removals in this window.

## Active Observation Targets

The model target remains active-observation based:

`P(direct LP removal within next N active observations | pool state up to observation O)`

| Horizon | Rows | Positives | Positive share |
| --- | ---: | ---: | ---: |
| 1 active observation | 68,588 | 576 | 0.8398% |
| 2 active observations | 68,588 | 1,152 | 1.6796% |
| 3 active observations | 68,588 | 1,727 | 2.5179% |
| 5 active observations | 68,588 | 2,833 | 4.1305% |
| 10 active observations | 68,588 | 5,199 | 7.5800% |

## Strategy Designs

### 1. Mined LP Approval Immediate Exit

Use a mined-chain LP approval signal, not only replayed mempool risk events.

Trigger:

- Pool is held.
- New LP approval is mined for the pool LP token.
- Generic LP approval amount is large, for example capped approval percent >= `90%`.
- Approval owner is creator or current LP top holder.

Action:

- If `lead >= 2` historically, next-block exit is clean.
- If `lead == 1`, submit a priority exit but treat it as a race, not guaranteed execution.

Edge:

- `571 / 576` eligible direct LP scams had pre-removal approval.
- `511 / 576` had two-plus blocks and effective sell at approval.

### 2. LP Approval Launch Gate

Use LP approval as a no-entry or instant de-risk condition.

Trigger:

- LP approval appears within the first `1-5` blocks after trading is enabled.
- Approval is effectively full LP supply.
- LP top holder is concentrated.

Action:

- Avoid new entry if not already in.
- If already in, exit or reduce immediately.

Edge:

- `82.12%` of direct LP scams with pre-removal approval had the final pre-removal approval within `5` blocks after trading enabled.

### 3. Direct LP Active-Horizon Risk Gate

Train/use a risk model for direct LP removal within `1, 2, 3, 5, 10` active observations.

Trigger features:

- LP approval age.
- Generic LP approval percent.
- LP holder concentration.
- Creator/current-owner approval.
- Liquidity drawdown from max.
- Price-to-initial ratio.
- Activity density and transfer ratios.

Action:

- Do not buy or exit when predicted near-future risk crosses threshold.

Edge:

- The positive share rises from `0.8398%` for 1 active observation to `7.58%` for 10 active observations, giving a usable ranking target rather than a broad lifetime scam label.

### 4. V2 Pair-Balance Backdoor Guard

Direct LP approval does not cover the second largest scam family.

Trigger:

- Protocol is V2.
- Pair-balance drain indicators appear: reserve changes without normal LP burn path, abnormal token or denom flow out of pair, or pair balance/backdoor label candidates.

Action:

- Create a separate near-future target for pair-balance backdoor drain.
- Gate or exit V2 pools when reserve/balance behavior diverges from normal swap/burn activity.

Edge:

- Pair-balance backdoor drain is `461 / 1,280` scam labels (`36.02%`).
- Median scam age is `31` blocks, faster than direct LP removal median `74`.

### 5. Protocol-Specific Scam Guardrails

Use different guards by protocol instead of one generic rule.

Trigger:

- V2: prioritize direct LP approval and pair-balance backdoor signals.
- V4: prioritize reserve-dump/unknown-reserve-drain signals.
- V3: treat direct LP removal as rare in this window.

Action:

- Use protocol-conditioned risk thresholds and feature sets.

Edge:

- V2 scams are mostly direct LP removal and pair-balance backdoor.
- V4 scams in this window are `unknown_reserve_drain` and `reserve_dump_drain`, not direct LP approval removal.

## Implementation Notes

The historical strategy run that only exited twice on LP approval did not use this mined-chain approval feed. It replayed stored `mempool_signal` observations, and the source run only had a small number of LP approval risk events. The mined-chain token analytics feed sees many more approval events and should be the source for a historical LP approval exit strategy.

Before using `lp_approved_pct_as_of` in UI or models, split the feature into:

- generic LP approval percent, based on max approved LP amount capped by LP total supply;
- router-approved LP percent, which requires known router configuration.
