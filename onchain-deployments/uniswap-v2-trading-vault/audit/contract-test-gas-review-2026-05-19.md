# Contract Test And Gas Review - 2026-05-19

Scope: `UniswapV2TradingVault` source, local unit tests, deterministic gas
benchmarks, and mainnet-fork gas benchmarks for the deployment run
`runs/2026-05-19-mainnet-v1`.

No deployment transaction was sent during this review.

## Contract Surface Reviewed

- Immutable config: `owner`, `treasury`, `weth`, `uniswapV2Router`.
- Access control: `buyV2ExactEthForTokens`,
  `emergencySellV2ExactTokensForEth`, `rescueToken`, and `rescueEth` are
  owner-only.
- Reentrancy: every owner state-changing method uses the vault's non-reentrant
  guard.
- Buy path: V2 supporting-fee-on-transfer router buy, vault-held token output,
  net token delta check, no sell-router approval created.
- Emergency sell path: exact approval, V2 supporting-fee-on-transfer router
  sell, approval cleared, ETH proceeds forwarded to treasury.
- Rescue path: owner-only token and ETH rescue with zero address and zero
  amount rejection.

## Tests Run

```text
forge test --match-contract UniswapV2TradingVaultTest -vv
13 passed; 0 failed; 0 skipped

forge test
26 passed; 0 failed; 0 skipped

forge test --match-contract UniswapV2TradingVaultGasTest --gas-report
7 passed; 0 failed; 0 skipped

forge test --match-contract UniswapV2TradingVaultForkGasTest \
  --fork-url http://127.0.0.1:8545 \
  --fork-block-number 25129486 \
  --gas-report
6 passed; 0 failed; 0 skipped
```

`scripts/01_build_and_hash.sh` and `scripts/02_fork_rehearsal.sh` were also
rerun for `runs/2026-05-19-mainnet-v1`.

## Coverage Notes

- Added unit coverage for constructor config and zero constructor addresses.
- Added unit coverage for buy zero token, zero ETH, and expired deadline.
- Added unit coverage for emergency-sell zero token, zero amount, and expired
  deadline.
- Added unit coverage for direct ETH receive and owner ETH rescue.
- Added unit coverage for rescue zero address and zero amount rejects.
- Existing tests already covered no standing sell approval after buy, exact
  approval and clear after emergency sell, fee-on-transfer buy/sell behavior,
  min-output reverts, owner-only controls, and treasury payout.

## Gas Benchmarks

Local deterministic gas report:

| Path | Gas |
| --- | ---: |
| Direct router buy to EOA | 115,169 |
| Vault buy to vault | 110,742 |
| Standalone approve | 50,375 |
| Direct router sell after preapproval | 84,855 |
| Vault emergency sell | 118,748 |
| Fee-on-transfer vault buy | 135,245 |
| Fee-on-transfer vault emergency sell | 134,389 |

Mainnet fork gas report at block `25129486`:

| Path | Gas |
| --- | ---: |
| Direct router buy to EOA | 124,125 |
| Vault buy to vault | 130,272 |
| Standalone approve | 62,585 |
| Direct router sell after preapproval | 95,465 |
| Vault emergency sell | 134,506 |
| Fee-on-transfer RFI vault buy and emergency sell | 389,938 |

Contract deployment gas report:

| Metric | Value |
| --- | ---: |
| Deployment gas | 769,564 |
| Deployed size | 3,831 bytes |

The normal fork buy overhead is small versus direct router buy
(`130,272 - 124,125 = 6,147` gas). The vault emergency sell is cheaper than
doing a separate standalone approve plus direct sell in the same comparison
(`134,506` gas versus `62,585 + 95,465 = 158,050` gas), because the vault
combines exact approval, swap, approval clear, and treasury transfer in one
owner call.

The contract function-level gas table has a high max buy cost because it also
includes the fee-on-transfer RFI path. Treat the per-test rows above as the
more useful deployment planning numbers.

## Result

No blocking Solidity issues were found in this pass. The contract-side tests
and gas profile are acceptable for the next deployment gate.

Still pending before production broadcast:

- exact deployed target allowlist in Kartal;
- final simulation of exact buy and emergency-sell calldata against current
  state;
- post-deploy immutable readback;
- operator/reviewer signoff.
