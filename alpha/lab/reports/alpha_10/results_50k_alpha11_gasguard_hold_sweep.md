# Alpha11 Gas-Guard Hold Sweep, 50k Blocks

Run ID: `alpha11-risk-atlas-50k-gasguard-hold-sweep-25067915-25117914-20260518`

Range: `25,067,915 -> 25,117,914`

Replay source: `risk-atlas-alpha10-50k-25067915-25117914-20260518`

Status: `completed`

Started: `2026-05-18 21:38:37+02`

Stopped: `2026-05-18 22:43:45+02`

## Design Questions

1. Does shortening the leader hold window to 12 blocks reduce early liquidity-removal loss without surrendering too much of the profitable tail?
2. Does keeping the 15-block leader window but using only the simulator gas/proceeds guard recover the old leader while avoiding truly uneconomic sells?
3. Does extending the hold window to 20 blocks capture more post-approval upside once uneconomic drained-pool sells are cancelled?

## Strategy Design

All three variants use the alpha10 leader entry surface:

- Uniswap V2 only.
- Enter eligible pools after the LP approval gate.
- Exit on LP approval, direct liquidity removal, max hold, or failed retry exhaustion.
- Defer buy-confirm-block LP approval exits to max-hold, matching the alpha10 leader finding.
- Retry exits up to 3 times at 1-block intervals.
- Set `min_sell_pool_denom_reserve = 0`, so the fixed `0.01 WETH` no-sell cutoff is disabled. Sells are still cancelled by the simulator when expected proceeds are less than or equal to gas.

The only intentional comparison variable is max active hold:

| Strategy | Max Hold |
|---|---:|
| `alpha11-01-v2-hold12-retry3-gasguard` | 12 blocks |
| `alpha11-02-v2-hold15-retry3-gasguard` | 15 blocks |
| `alpha11-03-v2-hold20-retry3-gasguard` | 20 blocks |

## Headline Results

| Run / Strategy | Trades | Closed | Open or Exposed | Failed | Realized ETH | Unrealized ETH | Total ETH | Losers | Loss ETH |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Previous alpha10 leader: `alpha10-10-v2-hold15-retry3` | 1,450 | 1,429 | 21 | 8 | 10.652545 | -0.074258 | 10.578287 | 403 | -2.367100 |
| Dust-fix v2 leader: `alpha10-10-v2-hold15-retry3` | 1,450 | 1,246 | 204 | 8 | 12.444551 | -1.904258 | 10.540293 | 404 | -2.375566 |
| `alpha11-01-v2-hold12-retry3-gasguard` | 1,450 | 1,354 | 96 | 8 | 8.994897 | -0.845764 | 8.149133 | 375 | -2.091769 |
| `alpha11-02-v2-hold15-retry3-gasguard` | 1,450 | 1,334 | 116 | 8 | 11.611855 | -1.024258 | 10.587597 | 403 | -2.357790 |
| `alpha11-03-v2-hold20-retry3-gasguard` | 1,450 | 1,301 | 149 | 9 | 14.634158 | -1.332479 | 13.301679 | 470 | -3.040146 |

## Exit Mix

| Strategy | Exit Reason | Submit-Sell Decisions |
|---|---|---:|
| hold12 | `exit.failed_retry` | 13 |
| hold12 | `exit.liquidity_removal` | 138 |
| hold12 | `exit.lp_approval` | 601 |
| hold12 | `exit.max_hold_active_blocks` | 708 |
| hold15 | `exit.failed_retry` | 13 |
| hold15 | `exit.liquidity_removal` | 170 |
| hold15 | `exit.lp_approval` | 608 |
| hold15 | `exit.max_hold_active_blocks` | 669 |
| hold20 | `exit.failed_retry` | 15 |
| hold20 | `exit.liquidity_removal` | 205 |
| hold20 | `exit.lp_approval` | 625 |
| hold20 | `exit.max_hold_active_blocks` | 617 |

Uneconomic sell cancellations:

| Strategy | Cancelled Uneconomic Sells |
|---|---:|
| hold12 | 77 |
| hold15 | 95 |
| hold20 | 125 |

## First Exit Reason Attribution

This attributes each position to the first sell reason observed for the same strategy and pool.

| Strategy | First Exit Reason | Trades | Closed | Not Closed | Total ETH | Losers | Loss ETH |
|---|---|---:|---:|---:|---:|---:|---:|
| hold12 | `exit.liquidity_removal` | 133 | 66 | 67 | -1.289185 | 132 | -1.318714 |
| hold12 | `exit.lp_approval` | 598 | 585 | 13 | 0.134936 | 196 | -0.441501 |
| hold12 | `exit.max_hold_active_blocks` | 708 | 703 | 5 | 9.299965 | 41 | -0.309519 |
| hold12 | no sell decision | 11 | 0 | 11 | 0.003417 | 6 | -0.022035 |
| hold15 | `exit.liquidity_removal` | 164 | 81 | 83 | -1.598117 | 163 | -1.627646 |
| hold15 | `exit.lp_approval` | 604 | 590 | 14 | 0.096093 | 201 | -0.487308 |
| hold15 | `exit.max_hold_active_blocks` | 669 | 663 | 6 | 12.086133 | 32 | -0.209383 |
| hold15 | no sell decision | 13 | 0 | 13 | 0.003488 | 7 | -0.033454 |
| hold20 | `exit.liquidity_removal` | 199 | 107 | 92 | -1.942799 | 198 | -1.972328 |
| hold20 | `exit.lp_approval` | 619 | 602 | 17 | 0.199240 | 208 | -0.541945 |
| hold20 | `exit.max_hold_active_blocks` | 617 | 592 | 25 | 15.040017 | 56 | -0.492380 |
| hold20 | no sell decision | 15 | 0 | 15 | 0.005221 | 8 | -0.033492 |

## Hold-Time Read

| Strategy | Hold Bucket | Trades | Total ETH | Losers | Loss ETH |
|---|---|---:|---:|---:|---:|
| hold12 | `<=5` | 484 | 0.231690 | 136 | -0.091633 |
| hold12 | `6-10` | 86 | -0.298749 | 46 | -0.336613 |
| hold12 | `11-15` | 58 | 0.485720 | 33 | -0.250507 |
| hold12 | `16-20` | 187 | 2.151071 | 26 | -0.241976 |
| hold12 | `>20_or_open` | 635 | 5.579400 | 134 | -1.171040 |
| hold15 | `<=5` | 484 | 0.231690 | 136 | -0.091633 |
| hold15 | `6-10` | 86 | -0.298749 | 46 | -0.336613 |
| hold15 | `11-15` | 37 | 0.095313 | 31 | -0.240988 |
| hold15 | `16-20` | 42 | 0.194246 | 24 | -0.194477 |
| hold15 | `>20_or_open` | 801 | 10.365098 | 166 | -1.494079 |
| hold20 | `<=5` | 484 | 0.231690 | 136 | -0.091633 |
| hold20 | `6-10` | 86 | -0.298749 | 46 | -0.336613 |
| hold20 | `11-15` | 32 | -0.224288 | 29 | -0.231422 |
| hold20 | `16-20` | 23 | 0.168450 | 20 | -0.182608 |
| hold20 | `>20_or_open` | 825 | 13.424576 | 239 | -2.197870 |

## Read

Hold12 is too conservative. It lowers losers and loss ETH, but it gives up too much max-hold upside and finishes at `8.149133 ETH`.

Hold15 with gas-guard-only behavior is the cleanest continuity check against the alpha10 leader. It slightly improves total PnL versus the original leader (`10.587597 ETH` vs `10.578287 ETH`) and avoids the excessive open exposure from the fixed `0.01 WETH` dust cutoff run.

Hold20 is the current alpha candidate from this sweep. It wins total PnL by a large margin at `13.301679 ETH`, driven by `15.040017 ETH` from max-hold exits. The cost is real: liquidity-removal exits lose `-1.942799 ETH`, loser count rises to 470, and open/exposed positions rise to 149.

The next validation should focus on hold20's max-hold winners and the 199 first-exit liquidity-removal positions. The question is whether the extra max-hold upside is stable without lookahead, or whether a pre-removal signal can keep most of the `15.04 ETH` max-hold profit while cutting the `-1.94 ETH` direct-removal loss.
