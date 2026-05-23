# Gamma 10 Profit Review

## Profit Surface To Protect

For the 50K alpha_10 leader:

| Metric | Value |
| --- | ---: |
| Winning trades | 1,047 |
| Winner PnL ETH | 12.945387 |
| Losing trades | 403 |
| Loser PnL ETH | -2.367100 |
| Net PnL ETH | 10.578287 |
| Top 1 winner share | 2.8323% |
| Top 5 winner share | 5.8959% |

The policy is not just one extreme winner. That gives room to reduce losses,
but the same-confirm LP approval cohort generated most of the inspected PnL, so
a blunt quarantine can still destroy the edge.

## Profit Questions

1. Do hold5/8/10/12 exits give up the launch moves that hold15 captured?
2. Does hold20 improve upside once buy-confirm deferral remains enabled?
3. Does 5x take-profit capture enough of the visible launch acceleration?
4. Does 0.85 stop-loss reduce direct LP race loss without cutting normal launch
   volatility?
5. Do retries have value at hold12, where an earlier sell attempt might be more
   likely to survive removal timing?

## Profit Review After Backtest

After the gamma_10 run, compare each policy on:

1. total PnL;
2. realized PnL;
3. open and failed positions;
4. winner PnL lost versus `gamma10-09-v2-hold15-retry3`;
5. loss PnL saved versus `gamma10-09-v2-hold15-retry3`;
6. top-one/top-five winner concentration.

## Gamma 10 Result Update

The total-PnL leader is `gamma10-07-v2-hold20-buy-confirm`:

| Metric | Value |
| --- | ---: |
| Winning trades | 979 |
| Winner PnL ETH | 16.312296 |
| Total PnL ETH | 13.239751 |
| Top 1 winner share | 2.1983% |
| Top 5 winner share | 5.2501% |

The profit improvement is not caused by a single outlier. The open question is
whether the larger open exposure is acceptable; profit quality passes the first
concentration check, but risk quality still needs the open-position audit.
