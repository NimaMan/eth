# 10 Hold15 Stop-Loss 0.85 Take-Profit 5x

Strategy:

```text
gamma10-10-v2-hold15-sl85-tp5x
```

Question: can mark-to-market drawdown and a moderate profit cap improve the
race loss bucket without removing too much launch upside?

Expected read: this is diagnostic unless it beats hold15 retry on total and
realized PnL. If it saves loss but gives up too much winner PnL, drawdown is
not a sufficient replacement for better ordering/race handling.
