# Baygus V2 Mode A Benchmarks

This folder records gas evidence for the V2 Mode A trading vault.

Mode A means:

```text
buy:
  ETH -> token through the vault
  vault holds token
  no sell-router allowance remains

emergency sell:
  approve exact amount
  sell token -> ETH
  clear allowance
  send ETH to treasury
```

## Test Layout

```text
contracts/test/bench/
  V2GasBenchBase.sol
  BaygusTradingVaultGas.t.sol
  BaygusTradingVaultForkGas.t.sol

contracts/test/fixtures/
  V2MainnetFixtures.sol
```

`BaygusTradingVaultGas.t.sol` uses local mocks. It is useful for regression
tracking, but it is not a production gas estimate.

`BaygusTradingVaultForkGas.t.sol` uses mainnet Uniswap V2 router, WETH, and USDC
on a fork. It no-ops when run without fork code at the mainnet router address.

## Commands

Run deterministic local benchmarks:

```bash
cd /home/nima/code/crypto/blockchains/eth/solidity/baygus-executor/contracts
forge test --match-contract BaygusTradingVaultGasTest --gas-report
forge snapshot --match-contract BaygusTradingVaultGasTest \
  --snap ../benchmarks/snapshots/v2-mode-a-local.gas-snapshot
```

Run mainnet fork benchmarks:

```bash
cd /home/nima/code/crypto/blockchains/eth/solidity/baygus-executor/contracts
MAINNET_RPC_URL="$MAINNET_RPC_URL" forge test \
  --match-contract BaygusTradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --gas-report
```

For repeatable reports, add a fixed fork block:

```bash
MAINNET_RPC_URL="$MAINNET_RPC_URL" forge test \
  --match-contract BaygusTradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --fork-block-number <block> \
  --gas-report
```

## Metrics

Track these functions:

```text
testGas_*_DirectRouterBuyToEoa
testGas_*_VaultBuyToVault
testGas_*_StandaloneApprove
testGas_*_DirectRouterSellPreapproved
testGas_*_VaultModeAEmergencySell
```

Interpretation:

```text
vault_buy_extra_gas =
  VaultBuyToVault - DirectRouterBuyToEoa

mode_a_sell_extra_gas =
  VaultModeAEmergencySell - DirectRouterSellPreapproved

two_tx_sell_reference =
  StandaloneApprove + DirectRouterSellPreapproved
```

`two_tx_sell_reference` is not exactly the same as two separate public
transactions because transaction envelope/base-fee accounting happens outside
these function measurements. It is still useful as a lower-bound comparison for
how much execution gas Mode A avoids by merging approve+sell into one top-level
transaction.

## Decision Rule

Mode A is acceptable when the buy gas saved by not pre-approving is worth the
extra emergency-sell gas under the live gas-rank model.

If Mode A emergency sells cannot rank fast enough in scam-exit cases, compare
against a future Mode B benchmark:

```text
buyAndApprove:
  buy token
  approve exact sell amount in the same buy tx

emergency sell:
  sell-only
```
