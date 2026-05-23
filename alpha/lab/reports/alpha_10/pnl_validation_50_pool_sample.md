# Alpha 10 50-Pool PnL/Event Audit

Date: 2026-05-18

Result set: `historical-25067915-25117914`
Strategy: `alpha10-10-v2-hold15-retry3`
Replay run: `risk-atlas-alpha10-50k-25067915-25117914-20260518`

## Goal

Validate a deterministic 50-pool sample at two levels:

1. First-degree validation: no obvious information leakage in the event chronology. Buy/sell decisions must be explainable by events available at or before the decision block.
2. Second-degree validation: PnL, fills, gas, snapshots, and strategy behavior must reconcile with the persisted trade/event rows.

## Questions Asked

- Did every sampled buy submission have a same-block `submit_buy` decision?
- Did every sampled sell submission have a same-block `submit_sell` decision?
- Did any sampled pool have an LP approval visible before the buy submission?
- For `exit.lp_approval`, was an LP approval risk event available no later than the sell decision block?
- For `exit.liquidity_removal`, was a liquidity-removal risk event available no later than the sell decision block?
- Did buy confirmation occur one block after buy submission?
- Did terminal sell confirmation occur one block after the final sell submission, including retry cases?
- Did every sampled trade use `UNISWAP-V2`, matching the strategy suite?
- Does `entry_cost_eth` match the buy-confirmed filled ETH amount?
- Does `exit_value_eth` match the sell-confirmed filled ETH amount for closed trades?
- Does `gas_cost_eth` match the sum of gas costs from trade events?
- Does `realized_pnl_eth = exit_value_eth - entry_cost_eth - gas_cost_eth` for closed trades?
- Does `total_pnl_eth = realized_pnl_eth + unrealized_pnl_eth` for all sampled trades?
- For non-closed trades, does `total_pnl_eth = current_value_eth - entry_cost_eth - gas_cost_eth`?
- Does each latest trade row match the latest persisted snapshot pointer?

## Sample Design

The result has `1450` trades and `1450` unique pools, so each sampled trade is a unique pool.

Sample buckets:

| bucket | pools | sample PnL ETH |
| --- | ---: | ---: |
| top_winner | 8 | 0.777571913290 |
| worst_loser | 10 | -0.113414075927 |
| liquidity_removal_exit | 8 | -0.079920052148 |
| lp_approval_exit | 8 | 0.003016046865 |
| max_hold_exit | 6 | 0.126569898081 |
| open_position | 5 | -0.004543419442 |
| sell_failed | 1 | -0.010163960772 |
| deterministic_fill | 4 | 0.002328832824 |

## Audit Result

Hard-failure checks on the 50-pool sample:

| check | violations |
| --- | ---: |
| non-V2 protocol | 0 |
| buy decision block differs from buy submission block | 0 |
| buy decision event key uses a future block | 0 |
| buy confirmation not one block after buy submission | 0 |
| sell decision block differs from sell submission block | 0 |
| sell decision event key uses a future block | 0 |
| terminal sell confirmation not one block after final sell submission | 0 |
| LP approval visible before buy submission | 0 |
| LP approval exit without prior/local LP approval signal | 0 |
| liquidity removal exit without prior/local liquidity-removal signal | 0 |
| total PnL does not equal realized plus unrealized | 0 |
| closed realized PnL formula mismatch | 0 |
| open/failed total PnL formula mismatch | 0 |
| entry cost differs from buy-confirmed fill | 0 |
| exit value differs from sell-confirmed fill | 0 |
| gas differs from event gas sum | 0 |
| latest snapshot pointer mismatch | 0 |
| missing final closed snapshot | 0 |

Raw interpretation flags:

| flag | count | interpretation |
| --- | ---: | --- |
| max-hold exit with LP approval already visible by sell decision | 10 | Expected for this policy when LP approval appears in the buy-confirmation block. The strategy setting `defer_buy_confirm_block_lp_approval_to_max_hold` intentionally does not exit immediately on that same-block signal. In every flagged sample, buy submission was one block before the LP approval/buy confirmation block, so this is not entry leakage. |
| ROI not equal to total PnL divided by entry cost | 5 | Isolated to `sell_failed` trades. Their persisted ROI is `-1` while total PnL includes gas, so total/entry is less than `-1`. PnL is still reconciled; this is a ROI display/semantics caveat for failed sells. |

## All-Strategy Cross-Check

The same fill, gas, protocol, and accounting checks were also run over all `1450` trades in the strategy:

| check | violations |
| --- | ---: |
| non-V2 protocol | 0 |
| bad buy execution delay | 0 |
| bad terminal sell execution delay after retries | 0 |
| total PnL formula mismatch | 0 |
| closed realized PnL formula mismatch | 0 |
| open/failed total formula mismatch | 0 |
| entry fill mismatch | 0 |
| exit fill mismatch | 0 |
| gas sum mismatch | 0 |
| ROI mismatch excluding `sell_failed` | 0 |

One raw first-sell delay false positive exists if the check compares sell confirmation to the first sell submission instead of the final retry submission:

`trd_mpaidhnc_24exg_5lw` submitted sells at blocks `25093158`, `25093160`, and `25093162`; the first two failed and the final sell confirmed at `25093163`. This is valid retry behavior.

## Sample Pools

| no | bucket | trade | state | entry | exit | PnL ETH | sell reason |
| ---: | --- | --- | --- | ---: | ---: | ---: | --- |
| 1 | top_winner | `trd_mpae2vvt_24exg_xy` | sell_confirmed | 25073665 | 25073680 | 0.299613542725 | exit.max_hold_active_blocks |
| 2 | top_winner | `trd_mpagyivv_24exg_46e` | sell_confirmed | 25087609 | 25087647 | 0.055288646891 | exit.lp_approval |
| 3 | top_winner | `trd_mpahwbh0_24exg_567` | sell_confirmed | 25091263 | 25091294 | 0.057579579399 | exit.max_hold_active_blocks |
| 4 | top_winner | `trd_mpaj1sub_24exg_61b` | sell_confirmed | 25096574 | 25096597 | 0.052407699741 | exit.max_hold_active_blocks |
| 5 | top_winner | `trd_mpajfoz3_24exg_6lh` | sell_confirmed | 25098573 | 25098638 | 0.051646636195 | exit.max_hold_active_blocks |
| 6 | top_winner | `trd_mpakvtms_24exg_8bc` | sell_confirmed | 25105641 | 25105670 | 0.049836607727 | exit.max_hold_active_blocks |
| 7 | top_winner | `trd_mpam1wqa_24exg_94a` | sell_confirmed | 25109506 | 25109543 | 0.109396950652 | exit.lp_approval |
| 8 | top_winner | `trd_mpanb3fc_24exg_afj` | sell_confirmed | 25114568 | 25114601 | 0.101802249961 | exit.max_hold_active_blocks |
| 9 | worst_loser | `trd_mpaepuh9_24exg_20i` | buy_confirmed | 25077763 |  | -0.011418294659 | none |
| 10 | worst_loser | `trd_mpaf7p1g_24exg_2bz` | sell_failed | 25080327 |  | -0.010378989433 | exit.max_hold_active_blocks |
| 11 | worst_loser | `trd_mpaf9pbs_24exg_2d3` | sell_failed | 25080625 |  | -0.010218947988 | exit.max_hold_active_blocks |
| 12 | worst_loser | `trd_mpah2j7x_24exg_499` | sell_failed | 25088003 |  | -0.010651572700 | exit.max_hold_active_blocks |
| 13 | worst_loser | `trd_mpahk40w_24exg_4vx` | sell_confirmed | 25089883 | 25102437 | -0.012383295087 | exit.lp_approval |
| 14 | worst_loser | `trd_mpainoa0_24exg_5of` | sell_confirmed | 25094526 | 25102441 | -0.013121148033 | exit.lp_approval |
| 15 | worst_loser | `trd_mpaivccy_24exg_5tv` | sell_confirmed | 25095662 | 25095788 | -0.010236626516 | exit.liquidity_removal |
| 16 | worst_loser | `trd_mpajzidk_24exg_789` | sell_failed | 25101306 |  | -0.010326926905 | exit.lp_approval |
| 17 | worst_loser | `trd_mpak2km3_24exg_78j` | sell_confirmed | 25102125 | 25102447 | -0.013363437741 | exit.lp_approval |
| 18 | worst_loser | `trd_mpak3njr_24exg_78t` | sell_confirmed | 25102552 | 25102575 | -0.011314836865 | exit.max_hold_active_blocks |
| 19 | liquidity_removal_exit | `trd_mpaejqq5_24exg_1so` | sell_confirmed | 25076819 | 25076841 | -0.009960903711 | exit.liquidity_removal |
| 20 | liquidity_removal_exit | `trd_mpafok6a_24exg_2xz` | sell_confirmed | 25082270 | 25082287 | -0.010030706598 | exit.liquidity_removal |
| 21 | liquidity_removal_exit | `trd_mpafqb3z_24exg_308` | sell_confirmed | 25082441 | 25082452 | -0.009980746163 | exit.liquidity_removal |
| 22 | liquidity_removal_exit | `trd_mpaj3wg5_24exg_63u` | sell_confirmed | 25096898 | 25096907 | -0.010015930649 | exit.liquidity_removal |
| 23 | liquidity_removal_exit | `trd_mpakp1n9_24exg_7u3` | sell_confirmed | 25104970 | 25104974 | -0.010038758621 | exit.liquidity_removal |
| 24 | liquidity_removal_exit | `trd_mpakpxn6_24exg_7wb` | sell_confirmed | 25105035 | 25105039 | -0.010039729925 | exit.liquidity_removal |
| 25 | liquidity_removal_exit | `trd_mpamu5g8_24exg_9yu` | sell_confirmed | 25112537 | 25112559 | -0.009955121357 | exit.liquidity_removal |
| 26 | liquidity_removal_exit | `trd_mpant4qn_24exg_azk` | sell_confirmed | 25117005 | 25117024 | -0.009898155123 | exit.liquidity_removal |
| 27 | lp_approval_exit | `trd_mpadtper_24exg_o3` | sell_confirmed | 25070534 | 25070537 | -0.000321379399 | exit.lp_approval |
| 28 | lp_approval_exit | `trd_mpaev8k7_24exg_23l` | sell_confirmed | 25078583 | 25078585 | 0.000789224560 | exit.lp_approval |
| 29 | lp_approval_exit | `trd_mpahfvhs_24exg_4q1` | sell_confirmed | 25089303 | 25089309 | 0.000520094351 | exit.lp_approval |
| 30 | lp_approval_exit | `trd_mpahlm5b_24exg_4xb` | sell_confirmed | 25090073 | 25090076 | 0.000644662841 | exit.lp_approval |
| 31 | lp_approval_exit | `trd_mpaj42tw_24exg_64e` | sell_confirmed | 25096919 | 25096921 | 0.000668572162 | exit.lp_approval |
| 32 | lp_approval_exit | `trd_mpakg8lf_24exg_7kv` | sell_confirmed | 25103943 | 25103945 | 0.000765904760 | exit.lp_approval |
| 33 | lp_approval_exit | `trd_mpamhwwi_24exg_9ky` | sell_confirmed | 25111162 | 25111164 | 0.000157612515 | exit.lp_approval |
| 34 | lp_approval_exit | `trd_mpanocpu_24exg_at6` | sell_confirmed | 25116417 | 25116419 | -0.000208644926 | exit.lp_approval |
| 35 | max_hold_exit | `trd_mpafmj7r_24exg_2un` | sell_confirmed | 25082034 | 25082060 | 0.032859039122 | exit.max_hold_active_blocks |
| 36 | max_hold_exit | `trd_mpag7bj7_24exg_3lo` | sell_confirmed | 25084571 | 25084617 | 0.005739138865 | exit.max_hold_active_blocks |
| 37 | max_hold_exit | `trd_mpaglwi8_24exg_41c` | sell_confirmed | 25086153 | 25086186 | 0.030870377617 | exit.max_hold_active_blocks |
| 38 | max_hold_exit | `trd_mpalwh39_24exg_90o` | sell_confirmed | 25109073 | 25109106 | 0.012973626721 | exit.max_hold_active_blocks |
| 39 | max_hold_exit | `trd_mpamki8t_24exg_9oa` | sell_confirmed | 25111410 | 25111439 | 0.027839014333 | exit.max_hold_active_blocks |
| 40 | max_hold_exit | `trd_mpao1ram_24exg_b8c` | sell_confirmed | 25117677 | 25117703 | 0.016288701423 | exit.max_hold_active_blocks |
| 41 | open_position | `trd_mpaeg0ee_24exg_1lq` | buy_confirmed | 25076137 |  | 0.011489896661 | none |
| 42 | open_position | `trd_mpagvglz_24exg_45a` | buy_confirmed | 25087272 |  | -0.010069533377 | none |
| 43 | open_position | `trd_mpagzpvj_24exg_47s` | buy_confirmed | 25087716 |  | -0.003633365418 | none |
| 44 | open_position | `trd_mpaj0j4g_24exg_5zx` | buy_confirmed | 25096381 |  | -0.004615898776 | none |
| 45 | open_position | `trd_mpam8p0k_24exg_9b3` | buy_confirmed | 25110174 |  | 0.002285481469 | none |
| 46 | sell_failed | `trd_mpagqo2q_24exg_44g` | sell_failed | 25086728 |  | -0.010163960772 | exit.lp_approval |
| 47 | deterministic_fill | `trd_mpafdwrz_24exg_2j7` | sell_confirmed | 25081037 | 25081041 | 0.000066900707 | exit.lp_approval |
| 48 | deterministic_fill | `trd_mpajbgj6_24exg_6f3` | sell_confirmed | 25097837 | 25097840 | 0.001743344288 | exit.lp_approval |
| 49 | deterministic_fill | `trd_mpajgdno_24exg_6ml` | sell_confirmed | 25098703 | 25098706 | 0.000425330120 | exit.lp_approval |
| 50 | deterministic_fill | `trd_mpajo8dc_24exg_6xf` | sell_confirmed | 25099712 | 25099714 | 0.000093257710 | exit.lp_approval |

## Interpretation

No sampled pool showed entry leakage. The buy decision and buy submission blocks matched, and no sampled LP approval was visible before buy submission.

Risk exits were locally explainable. Every sampled `exit.lp_approval` had an LP approval risk event no later than the sell decision block. Every sampled `exit.liquidity_removal` had a liquidity-removal risk event no later than the sell decision block.

PnL reconciled in the sample and in the full strategy population. Entry cost, exit value, gas, realized PnL, unrealized PnL, and total PnL matched the persisted trade events/snapshots within `1e-15` ETH.

The remaining item is not a PnL calculation error: sell-failed ROI is stored as `-1`, while total PnL includes gas, so total PnL divided by entry cost is slightly below `-1`. We should decide whether failed-sell ROI should be kept as the intuitive "capital fully impaired" value or changed to include gas in the displayed ROI.
