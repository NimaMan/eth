# Trading Vault Benchmarks

This folder records gas evidence for trading vault candidates.

## Uniswap V2 Mode A

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
  UniswapV2TradingVaultGas.t.sol
  UniswapV2TradingVaultForkGas.t.sol

contracts/test/fixtures/
  V2MainnetFixtures.sol
```

`UniswapV2TradingVaultGas.t.sol` uses local mocks. It is useful for regression
tracking, but it is not a production gas estimate.

`UniswapV2TradingVaultForkGas.t.sol` uses mainnet Uniswap V2 router, WETH, and USDC
on a fork. It also includes an RFI fee-on-transfer smoke benchmark. It no-ops
when run without fork code at the mainnet router address.

## Commands

Run deterministic local benchmarks:

```bash
cd /home/nima/code/crypto/blockchains/eth/solidity/baygus-executor/contracts
forge test --match-contract UniswapV2TradingVaultGasTest --gas-report
forge snapshot --match-contract UniswapV2TradingVaultGasTest \
  --snap ../benchmarks/snapshots/v2-mode-a-local.gas-snapshot
```

Run mainnet fork benchmarks:

```bash
cd /home/nima/code/crypto/blockchains/eth/solidity/baygus-executor/contracts
MAINNET_RPC_URL="$MAINNET_RPC_URL" forge test \
  --match-contract UniswapV2TradingVaultForkGasTest \
  --fork-url "$MAINNET_RPC_URL" \
  --gas-report
```

For repeatable reports, add a fixed fork block:

```bash
MAINNET_RPC_URL="$MAINNET_RPC_URL" forge test \
  --match-contract UniswapV2TradingVaultForkGasTest \
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
testGas_Local_FeeOnTransferVaultBuyToVault
testGas_Local_FeeOnTransferVaultModeAEmergencySell
testGas_Fork_FeeOnTransferRfiVaultBuyAndEmergencySell
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

## Uniswap V4 Candidate

The V4 candidate has separate local and fork gas tests:

```text
contracts/test/bench/v4/
  V4GasBenchBase.sol
  UniswapV4TradingVaultGas.t.sol
  UniswapV4TradingVaultForkGas.t.sol
```

Run local mock benchmarks:

```bash
cd /home/nima/code/crypto/blockchains/eth/solidity/baygus-executor/contracts
forge test --match-contract UniswapV4TradingVaultGasTest --gas-report
```

Run the pinned mainnet fork comparison for the ETH/USDC 0.05% no-hook V4
fixture:

```bash
forge test \
  --match-contract UniswapV4TradingVaultForkGasTest \
  --fork-url http://127.0.0.1:8545 \
  --fork-block-number 25131251 \
  --gas-report
```

Track these functions:

```text
testGas_*_DirectUniversalRouterBuyToEoa
testGas_*_VaultBuyToVault
testGas_*_Erc20ApprovePermit2
testGas_*_Permit2ApproveUniversalRouter
testGas_*_DirectUniversalRouterSellPreapproved
testGas_*_VaultEmergencySellWithExactPermit2Lifecycle
```

Interpretation:

```text
v4_vault_buy_extra_gas =
  VaultBuyToVault - DirectUniversalRouterBuyToEoa

v4_vault_sell_vs_preapproved_direct =
  VaultEmergencySellWithExactPermit2Lifecycle - DirectUniversalRouterSellPreapproved

v4_two_tx_sell_reference =
  Erc20ApprovePermit2 + Permit2ApproveUniversalRouter + DirectUniversalRouterSellPreapproved
```
