# PnL Examples

Pool-scoped user PnL verification harnesses.

The first target is a Uniswap V2 token/WETH pool where we can replay the token
builder over a compact launch range and check token/denom conservation from the
new `eth_token::pnl` ledger.

The example includes traces by default. That matters for WETH pools because
router unwraps and ETH payouts are native transfers, not ERC-20 WETH transfer
logs. Use `--no-traces` only when you are checking reserve replay quickly and do
not need native-denom address attribution.

## HODL/WETH Fixture

Source pair:
https://dexscreener.com/ethereum/0x538F76361ad5e94f21dB670e07f3b4DfF186AF3F

Run:

```bash
cargo run -p eth_token --example uniswap_v2_pool_user_pnl_conservation -- \
  --known hodl_weth_may_2026
```

Use `--list-known` to print available fixtures. Override `--end` when checking a
larger local Reth range.

## Persistence

Database persistence lives in `eth_token_pnl_store` so `eth_token` remains a
pure calculation crate.

```bash
cargo run -p eth_token_pnl_store --example persist_uniswap_v2_pool_pnl -- \
  --run-id hodl-weth-pnl-v1 \
  --token 0x538F76361ad5e94f21dB670e07f3b4DfF186AF3F \
  --pool 0xb1440adcAc60dCd82d7F40205DaF5f2cC96Edc61 \
  --start 25106317 \
  --end 25106381 \
  --config /home/nima/code/crypto/blockchains/eth/config.toml
```

Set `databases.token_pnl.url` in `blockchains/eth/config.toml`.
