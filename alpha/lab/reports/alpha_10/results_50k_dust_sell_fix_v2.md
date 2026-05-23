# Alpha 10 Leader 50K Dust Sell Fix Backtest

Generated on 2026-05-18.

## Run

This reruns the current alpha10 leader policy on the same 50K block window as
the prior 50K result, after adding the no-submit dust sell guard and correcting
Risk Atlas direct-removal replay ordering so the liquidity-removal risk decision
sees the current pool snapshot.

```text
run id: alpha10-risk-atlas-50k-leader-dust-sell-fix-v2-25067915-25117914-20260518
baseline run id: alpha10-risk-atlas-50k-25067915-25117914-20260518
replay run id: risk-atlas-alpha10-50k-25067915-25117914-20260518
strategy suite: alpha-10-risk-atlas-leader
strategy: alpha10-10-v2-hold15-retry3
range: 25067915 to 25117914
execution delay: 1 block
status: completed
```

Command:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run --release -p eth_alpha_backtest --bin eth_alpha_backtest -- \
  --run-id alpha10-risk-atlas-50k-leader-dust-sell-fix-v2-25067915-25117914-20260518 \
  --replay-run-id risk-atlas-alpha10-50k-25067915-25117914-20260518 \
  --strategy-suite alpha-10-risk-atlas-leader \
  --from-block 25067915 \
  --to-block 25117914 \
  --execution-delay-blocks 1
```

## Result

| Run | Trades | Closed | Open/exposed | Failed | Realized ETH | Unrealized ETH | Total ETH | Losers | Loss ETH |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Prior leader | 1,450 | 1,429 | 21 | 8 | 10.652545 | -0.074258 | 10.578287 | 403 | -2.367100 |
| Dust sell fix v2 | 1,450 | 1,246 | 204 | 8 | 12.444551 | -1.904258 | 10.540293 | 404 | -2.375566 |

Delta versus prior leader:

```text
total PnL: -0.037994 ETH
realized PnL: +1.792006 ETH
unrealized PnL: -1.830000 ETH
```

## Sell Guard Effects

| Metric | Prior leader | Dust sell fix v2 |
| --- | ---: | ---: |
| Liquidity-removal submit-sell decisions | 178 | 1 |
| Strategy dust holds on liquidity-removal exits | 0 | 177 |
| Strategy dust holds on max-hold exits | 0 | 78 |
| Sell submitted reports | 1,460 | 1,285 |
| Uneconomic simulated sell cancellations | 0 | 12 |

The original reviewed trade
`trd_mpaivccy_24exg_5tv` no longer submits a liquidity-removal sell. In the
fixed run, the matching pool has:

```text
block 25095787
action: hold
reason: exit.liquidity_removal:pool_denom_reserve_below_min_sell_threshold:0.000091383258997499<0.01
```

## Read

The execution hygiene fix works: mined direct-removal exits that see less than
`0.01` WETH reserve mostly stop before sell submission. The broad reserve
threshold is not a performance improvement on this range, though. The 177 pools
where the new rule skipped a sell were worth:

```text
old total PnL: -1.729970 ETH
new total PnL: -1.775417 ETH
delta: -0.045447 ETH
```

So the rule avoided bad-looking mined-removal sells, but it also gave up enough
small recoveries that total PnL fell slightly. The better next policy is a
proceeds-vs-gas or lower reserve threshold, not a blanket `0.01 WETH` pool
reserve cutoff.
