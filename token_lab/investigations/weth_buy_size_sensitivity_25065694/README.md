# WETH Buy Size Sensitivity

Status: explained policy case. The current fixed-size baseline correctly keeps
these as failed entries; adaptive sizing or pre-entry size guards are separate
strategy work.

## Scope

Historical run: `hist-snipe-all-poolonly-15k-recipient-net-20260513-145954`

Range: `25,065,694..25,080,693`

This investigation covers the five WETH-denominated V2/V3 buy failures left
after stable-denom pools were excluded from the executable baseline.

## Finding

The five failures are not missing setup, unsupported protocol, or stable-denom
accounting problems. They are fixed-size entry failures: the configured
`0.01 ETH` buy fails, while `0.001 ETH` succeeds on the same route at both
`block - 1` and the decision block.

This is a strategy sizing issue. With a fixed `0.01 ETH` order, the historical
backtest is correct to leave these as failed buys. A different strategy could
choose adaptive sizing or skip pools that cannot support the target order.

## Classified Entries

| Token | Block | Protocol | Failure at `0.01 ETH` | Probe result |
| --- | ---: | --- | --- | --- |
| `0x87AEA3...0e8754` | 25,074,266 | Uniswap V2 | `TransferHelper: TRANSFER_FROM_FAILED` | `0.001 ETH` buys and sells |
| `0xF02757...9E4a18` | 25,074,295 | Uniswap V2 | `UniswapV2: TRANSFER_FAILED` | `0.001 ETH` buys and sells |
| `0xD2D3bB...77815F` | 25,077,800 | Uniswap V2 | `UniswapV2: TRANSFER_FAILED` | `0.001 ETH` buys and sells |
| `0x5f5B88...454674` | 25,074,522 | Uniswap V3 | `TF` | `0.001 ETH` buys and sells |
| `0x821f85...B6F333` | 25,080,306 | Uniswap V3 | `TF` | `0.001 ETH` buys and sells |

For the representative V2 and V3 probes rerun after the WETH/native-only
baseline:

- `0.01 ETH` failed at `block - 1` and the decision block;
- `0.001 ETH` bought, approved, sold, and returned nonzero WETH;
- `0.0001 ETH` bought and sold but returned zero recorded denom output because
  the round trip was below useful output/dust precision.

## Required Strategy Work

Choose one explicit policy before using these failures as a strategy signal:

- fixed-size baseline: keep them as failed entries and count them as rejected
  executable opportunities;
- adaptive sizing: retry down to a configured minimum size and record the final
  executable size in the position;
- pre-entry sizing guard: skip pools where same-route simulation cannot support
  the target order.

The current historical baseline uses the first policy.

## Probe Command Shape

```bash
cargo run -q -p tx_processor --example probe_pool_buy_sell -- \
  --token <TOKEN> \
  --pool <POOL> \
  --protocol <uniswap-v2|uniswap-v3> \
  --fee-tier <V3_FEE_IF_NEEDED> \
  --block <BLOCK> \
  --denom 0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2 \
  --denom-decimals 18 \
  --token-decimals 18 \
  --amount-wei 10000000000000000,1000000000000000,100000000000000
```
