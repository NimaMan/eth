# Loss Review

## Loss Surface

The original all-protocol baseline has:

| Metric | Value |
| --- | ---: |
| Losing trades | 229 |
| Total loss ETH | -1.491478 |
| Worst single loss ETH | -0.013363 |
| Open/unmapped loss ETH | -0.389540 |
| Liquidity-removal exit loss ETH | -0.840352 |
| LP-approval exit loss ETH | -0.236325 |
| Max-hold exit loss ETH | -0.025261 |

The worst losses are usually near full entry loss plus gas, not deep negative
position sizing. That means reducing loser count can matter more than shaving a
few percent from each loser.

The best actual alpha_10 20K candidate,
`alpha10-10-v2-hold15-retry3`, has:

| Metric | Value |
| --- | ---: |
| Losing trades | 179 |
| Total loss ETH | -1.087625 |
| Liquidity-removal exit loss ETH | -0.820094 |
| LP-approval exit loss ETH | -0.234033 |
| Max-hold exit loss ETH | -0.025116 |
| Open/unmapped loss ETH | -0.008383 |

Compared with the all-protocol baseline, V2-only selection removes 50 losing
trades and about `0.403853 ETH` of loss. The remaining loss is still dominated
by late LP approval and direct-removal races.

The completed 50K best candidate,
`alpha10-10-v2-hold15-retry3`, has:

| Metric | Value |
| --- | ---: |
| Losing trades | 403 |
| Total loss ETH | -2.367100 |
| Liquidity-removal exit loss ETH | -1.708417 |
| LP-approval exit loss ETH | -0.494525 |
| Max-hold exit loss ETH | -0.130704 |
| Open/unmapped loss ETH | -0.033454 |

Compared with the 50K all-protocol baseline, V2-only hold15 removes 114 losing
trades and about `0.784963 ETH` of loss. The same loss mechanism remains:
direct LP approval often appears too late for clean next-block execution.

## Repeated Loss Signals

| Signal | Interpretation | Policy response |
| --- | --- | --- |
| `mark_to_market_below_entry_before_exit` | The pool showed drawdown before the final exit. | Test stop-loss and drawdown-speed exits. |
| `lp_approval_visible_by_sell_submit` | The strategy saw LP approval by sell time, but this did not guarantee a good exit. | Split clean lead from one-block race. |
| `sell_submitted_after_liquidity_removal` | Removal was already mined before the sell was submitted. | Do not count liquidity-removal exit as an executable edge. |
| `lp_approval_in_buy_confirm_block` | Buy was submitted before approval was visible, but confirmation block contained approval. | Compare immediate exit, defer-to-max-hold, and conservative exclusion. |
| `no_pool_risk_event_before_loss` | Direct LP warning did not explain the loss. | Add non-LP scam guards and mark these outside LP-only edge. |
| `gas_material_to_loss` | Gas explains a material share of small losses. | Add fee caps and avoid low expected-upside exits. |
| `lp_approval_lead_lte_1_block` | The exit was an ordering race. | Require lead greater than one block for clean historical credit. |

## Worst Samples

| Trade | Token | Buy C | LP App | Liq Rm | Sell S | Sell C | Exit | PnL ETH | Signals |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | --- |
| `trd_mpa7jclw_1slr7_hi` | `0x9F153e...D1668A` | 25102125 | 25102446 | 25102447 | 25102446 | 25102447 | LP approval | -0.013363 | one-block race, MTM below entry, gas |
| `trd_mpa7d9nw_1slr7_5a` | `0x802BAc...0F2ce0` | 25094526 | 25102440 | 25102441 | 25102440 | 25102441 | LP approval | -0.013121 | one-block race, MTM below entry |
| `trd_mpa7je93_1slr7_hk` | `0xed91b3...cB16Eb` | 25102552 | 25102647 | 25102648 | 25102574 | 25102575 | max hold | -0.011315 | later LP approval, MTM below entry |
| `trd_mpa7dv8o_1slr7_6o` | `0xa50f39...69FFB9` | 25095662 | 25095662 | 25095787 | 25095787 | 25095788 | liquidity removal | -0.010237 | buy-confirm approval, removal before sell |
| `trd_mpa7d26x_1slr7_58` | `0x60D762...A5f70e` | 25093950 | - | - | - | - | open | -0.010109 | no pool risk event |

## Loss-Prevention Hypotheses

1. **Race-aware LP exit:** Count one-block LP approval exits separately and do
   not credit them as clean exits unless sell confirmation precedes removal.
2. **Confirmed removal is a label, not an exit:** If removal is already mined,
   mark too-late and avoid pretending the strategy can recover.
3. **Buy-confirm approval quarantine:** Same-confirmation-block approval is not
   pre-buy evidence. Treat it as a separate policy branch with live-ordering
   caveats.
4. **Drawdown exit:** If mark-to-market crosses below entry before LP approval,
   stop-loss may prevent many losses, but it may also cut launch winners.
5. **Non-LP loss bucket:** Losses with no pool risk event require pair-balance,
   reserve-dump, or route-quality signals.
6. **Fee cap:** Gas-heavy losses should be prevented by execution guardrails,
   not by overfitting scam signals.
