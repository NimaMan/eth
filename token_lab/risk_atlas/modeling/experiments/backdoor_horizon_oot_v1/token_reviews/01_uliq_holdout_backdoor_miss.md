# Token Review 01 - ULIQ Holdout Backdoor Miss

Status: reviewed with server-backed token builder.

## Identity

- Token: `0x0ca72b24abf950be8bb145f5c68f7004485192b2`
- Pool: `0x57ca70b663ead85122589f345835d8f8209608dc`
- Name/symbol: `Uniliquid` / `ULIQ`
- Creator: `0x51e70799c78e6ccdd473a97a97ddfce577e5a643`
- Scam mechanism: `pair_balance_backdoor_drain`
- Scam block: `25144063`

## Token-Builder Replay

- API: `POST /eth/tokens/api/runs`
- Request:
  `{"start_block":25144040,"end_block":25144070,"history_limit":1000,"retention_mode":"keep_all","replace_active":false}`
- Local run id: `run-1`
- Blocks: `25144040..25144070`
- Source: `processed_block_disk_cache`
- Cache hits/misses: `31/0`
- Txs scanned/processed: `9169/9001`
- Token update reports: `58`
- Transaction failures: `0`
- Pool simulation failures: `0`
- Tracked tokens/pools: `2/2`

## Final Token/Pool State

- Token lifecycle: `INACTIVE_OTHER`
- Pool stage: `LIQUIDITY_REMOVED`
- Pool risk: `liquidity_removal`
- Scam label: `Backdoored Pair-Balance Drain`
- Final WETH reserve: `0.000047597440505978`
- Final token reserve: `211,930,025.6804654`
- Final pooled token supply: `21.1930%`
- Trading after drain: `can_buy=false`, `can_sell=false`
- Liquidity-removal tx:
  `0x07ee77181a3b47cca06b3dc20d46c1a02a60ca685430a3cca4cadc21e601c1b4`

## Event Sequence

| Block | Event | Evidence |
| ---: | --- | --- |
| `25144047` | Token and pool created; ownership renounced; trading initially works. | Creator txs create the token/pool, mint `1.0` WETH liquidity, run a tax check, and set owner to zero. |
| `25144047` | LP approval is present but not directly creator-removable. | `lp_approved_pct_as_of=100`, but `lp_removable_pct_as_of=0`; the last LP approval owner is the token contract, not the creator. |
| `25144057` | First backdoor warning. | Creator tx `0xc28c751c7215718dfa287b2f25e024019b1a0f72a8ad4529b26d0afff1e52b93` produces a control `transferFrom` signal after renounce, holder-to-burn flag, and no matching `Transfer` log. |
| `25144059` | Warning persists while trading still appears normal. | `can_buy=true`, `can_sell=true`, WETH reserve reaches `4.745464961237461`, and `last_pair_balance_backdoor_signal_to_as_of_chain_block_delta=2`. |
| `25144063` | Drain. | Creator-controlled activity transfers almost all paired tokens out, then swaps for `4.745417363796955` WETH, leaving dust WETH liquidity. |

## Key Observations

At block `25144057`, six chain blocks before the drain:

- `ownership_renounced=true`
- `can_buy=true`, `can_sell=true`
- `lp_removable_pct_as_of=0`
- `creator_lp_balance_pct_as_of=100`
- `control_transfer_from_count_in_block=1`
- `control_transfer_from_after_renounce_in_block=true`
- `control_transfer_from_holder_to_burn_in_block=true`
- `control_transfer_from_without_transfer_log_in_block=true`
- `pair_balance_backdoor_signal_in_block=true`

At block `25144059`, four chain blocks before the drain:

- `pair_balance_backdoor_signal_seen_as_of=true`
- `control_transfer_from_holder_to_burn_seen_as_of=true`
- `control_transfer_from_without_transfer_log_seen_as_of=true`
- `last_pair_balance_backdoor_signal_to_as_of_chain_block_delta=2`
- `denom_reserve=4.745464961237461`
- `price_to_initial_ratio=22.39165944514333`
- The iteration-01 4-block model score was only `1.14%`.

At block `25144063`, the drain mechanics are explicit:

- Suspicious pair-token transfer:
  `0x22e921be31f0ea850bea59a5491ab7036e25c5e5dfbbc98b67c496136dfafe75`
- Pair token transfer amount: `211,927,906.3802086` ULIQ
- Transfer direction:
  `pool -> 0x51e70799c78e6ccdd473a97a97ddfce577e5a643`
- Pooled-token share moved: `0.99999`
- Swap tx:
  `0x07ee77181a3b47cca06b3dc20d46c1a02a60ca685430a3cca4cadc21e601c1b4`
- WETH sold out: `4.745417363796955`
- Reserve ratio to max after drain: `0.001003%`

## Interpretation

This is not primarily an LP-removal setup. The LP approval signal exists at
launch, but the builder shows `lp_removable_pct_as_of=0` throughout the warning
window. The useful early warning is the control path:

```text
renounced owner
  -> creator/control tx calls token.transferFrom(holder, dead, amount)
  -> no matching Transfer log
  -> pair-balance backdoor flag appears
  -> pool remains buy/sell enabled for a few blocks
  -> creator drains paired token balance and swaps out WETH
```

The model missed this because the generic 4-block classifier mostly learned
direct LP-removal readiness. For ULIQ, the branch warning is a token-control
backdoor branch: first signal block, blocks since signal, signal persistence,
and interaction of holder-to-burn plus missing transfer log.

## Iteration 02 Feature Lesson

Add a separate token-control branch with:

- `first_pair_balance_backdoor_signal_chain_block`
- `blocks_since_first_pair_balance_backdoor_signal`
- `control_transfer_from_count_as_of`
- `control_transfer_from_after_renounce_count_as_of`
- `holder_to_burn_without_transfer_log_seen_as_of`
- `holder_to_burn_without_transfer_log_in_block`
- `backdoor_signal_seen_while_trading_enabled`
- `backdoor_signal_seen_and_lp_removable_pct_below_10`
- `creator_or_control_wallet_seen_in_network`

This case should become a regression target for iteration 02: a positive
4-block ULIQ row at block `25144059` should score materially higher than the
iteration-01 score of `1.14%`.
