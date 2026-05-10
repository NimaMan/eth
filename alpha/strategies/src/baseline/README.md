# Baseline Strategies

Simple, non-selective strategies used to establish a performance floor and
validate backtest infrastructure. They buy every eligible pool and serve as the
reference against which selective strategies are measured.

## Strategies

| Strategy | Path | Description |
|----------|------|-------------|
| Snipe All v1 | `snipe_all/` | Buy every eligible pool once (0.01 ETH). Exit on liquidity-removal / tax / LP-approval / scam signals. |

## Backtest Validation Findings

All measurements below use the honest backtest with `--skip-primed` (excludes
warmup observations) and modeled fills (500 bps slippage, 150k gas).

### Baseline: Buy All Eligible, Hold Forever (No Exits)

```
Positions:     205
Drained:       124 (-1.10 ETH)   ← 60% of positions go to zero
Zero:           27
Winners:        54 (+9.71 ETH)
Total:         +8.61 ETH unrealized
```

**Key insight**: Even the "dumb" baseline is profitable on paper because the
winners (54 positions) more than compensate for the losers (124 positions).
The average winner returns +180% ROI.

### Entry Timing Sanity Check

| Metric | Value |
|--------|-------|
| Bought at first observed block | 170 / 205 (83%) |
| Bought at later block | 35 / 205 (17%) |
| Average delay for late entries | ~1,558 blocks (~5.2 hours) |

**Price accuracy**: Our `entry_price` matches the pool's `price_denom_per_token`
at the buy block exactly (0.0000% difference across all positions).

**Token quantity**: `implied_tokens = cost_basis / entry_price`. For small
buys (0.01 ETH) against 1+ ETH liquidity pools, the approximation error is
<1% vs true Uniswap V2 swap math.

**Post-block state**: Pool updates in `strategy_observations` reflect the
post-block state (after all txs in the block are processed). The strategy
buys on `PoolUpdated` events, so `entry_price` already includes any sniper
activity in that block. This is the conservative/worst-case assumption.

### Exit Rule Quantification

Isolated backtests with each exit rule enabled individually:

| Exit Rule | Open | Closed | Realized | Unrealized |
|-----------|------|--------|----------|------------|
| **Baseline (no exits)** | 205 | 0 | — | +12.63 |
| **Liquidity removal only** | 110 | 95 | -0.0475 | +9.65 |
| **Scam only** | 110 | 95 | -0.0475 | +9.64 |
| **Tax only** | 203 | 2 | -0.0010 | +12.62 |
| **LP approval only** | 196 | 9 | -0.0045 | +12.45 |
| **All exits** | 108 | 97 | -0.0485 | +9.64 |

**Findings**:
- Liquidity removal accounts for **95 of 97 sells** when all exits are active.
- Tax and LP approval are negligible in this dataset (2 and 9 sells).
- Scam and liquidity removal have ~100% overlap (all scam targets also get
  liquidity-removed).

### Critical: Information Leakage in Mempool Signal Exits

The backtest replays mempool signals (e.g., liquidity removal) AFTER the
pool update for the signal block. This means the strategy "sees" the signal
and sells using the LAST pool state, which still shows healthy liquidity.

In reality, the removal tx is already in the mempool. By the time we react,
the pool may already be drained. The backtest **overestimates** exit
success.

**Example** (pool `0x1aca...c79a8`):

| Block | Event | Denom Reserve | Can Sell |
|-------|-------|---------------|----------|
| 25061037 | Pool update | 1.2 ETH | true |
| 25061039 | **Liquidity removal signal** | 1.203 ETH | true |
| 25061040 | Pool update | 0.00000082 ETH | **false** |

Backtest assumes we sell at block 25061039 prices (1.203 ETH liquidity).
Realistically, we might execute against the drained pool in block 25061040.

### Honest Baseline Fix

To make the baseline honest:

1. **`Position.drained` flag**: Set when `LiquidityRemoval` or `ScamConfirmed`
   risk events are received, even if the strategy doesn't exit.
2. **Snapshot-on-drain**: Append a position snapshot with `current_value = 0`
   and `unrealized = -cost_basis` so the final PnL reflects the drain even
   when no post-drain pool update exists.
3. **`unrealized_pnl()` zero-price fix**: When `current_price = 0` or
   `can_sell = false`, return `(0, -cost_basis)` instead of `(cost_basis, 0)`.

### Case Studies

#### Case 1: Winner — 357x Pump, Still Holding

- **Pool**: `0x8384...920e07`
- Entry: 0.01 ETH at block 25062508 (price = 1e-9 ETH/token)
- Peak: +5.31 ETH at block 25062730 (price = 532x entry)
- Final: +3.57 ETH at block 25062755 (price = 357x entry)
- **No exit signal ever fired**

**Lesson**: Need proactive take-profit rules. We're leaving gains on the table.

#### Case 2: Loser — Drained Before We Could React

- **Pool**: `0x1aca...c79a8`
- Entry: 0.01 ETH at block 25061037
- Signal: Liquidity removal at block 25061039
- Drained: Block 25061040 (denom_reserve = 0.00000082 ETH)
- Baseline final: -0.01 ETH
- All-exits realized: -0.0005 ETH (optimistic — assumes frontrun success)

**Lesson**: Exit rules help, but backtest is optimistic about frontrunning.

#### Case 3: Missed Peak — Pumped Then Drained

- **Pool**: `0x4Bfd...e8684`
- Peak unrealized: +0.0697 ETH (~7x)
- Final unrealized: -0.01 ETH (100% loss)
- **Missed profit**: +0.0797 ETH

**Lesson**: Proactive exit at 5x-10x would have captured +0.07 ETH that the
reactive exit rules missed entirely.

## Path Forward: Asymmetric Bets

The baseline shows the core opportunity: **winners produce 100x-500x returns
while losers lose 100%**. This is the classic asymmetric bet profile.

Next steps:
1. **Proactive exits** based on price ratio (e.g., sell 50% at 10x, 100% at 50x).
2. **Selective entry** using pool-quality scoring to reduce the 60% drain rate.
3. **Position sizing** to bet more on high-conviction pools.
4. **Honest mempool modeling** — model probability of successful frontrun vs
   failed exit for liquidity removal signals.
