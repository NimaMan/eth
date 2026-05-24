# V4 Trading Vault Simulation Examples

Examples for the V4 vault route rehearsal belong here. These examples are
intended to produce JSON that can be saved under
`deploy/onchain/uniswap-v4-trading-vault/simulations/reports/`.

## Direct Universal Router Baseline

Run the current direct-route baseline:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -q -p tx_simulator \
  --example uniswap_v4_trading_vault_direct_baseline \
  | tee deploy/onchain/uniswap-v4-trading-vault/simulations/reports/eth-usdc-500-no-hook-direct-baseline.json \
  | jq '{fixture_id, block, buy: .buy.transaction.gas_used, sell: .sell.transaction.gas_used, totals}'
```

The default fixture is:

`deploy/onchain/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json`

The direct baseline executes:

- native ETH -> USDC through the deployed Uniswap V4 Universal Router;
- ERC20 approval from the caller to Permit2;
- Permit2 allowance to the Universal Router;
- USDC -> native ETH through the deployed Universal Router.

It reports total route gas, allowance setup gas, effective gas price, priority
fee spend, balance deltas, and revert reasons when any step fails.

## Candidate Vault Rehearsal

After `forge build` has produced the local candidate artifact, run the simulator
comparison:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -q -p tx_simulator \
  --example uniswap_v4_trading_vault_candidate_rehearsal \
  | tee deploy/onchain/uniswap-v4-trading-vault/simulations/reports/eth-usdc-500-no-hook-candidate-vault-rehearsal.json \
  | jq '.comparison'
```

The rehearsal starts two forked simulation chains at the same block:

- direct Universal Router buy, ERC20 approval, Permit2 approval, sell;
- synthetic deployment of the candidate vault, vault buy, vault emergency sell.

Deployment gas is reported separately from route gas so direct route and vault
route execution can be compared without mixing one-time deployment cost into
trade cost.

## Required Before Deployment

- Permit2 allowance lifecycle rehearsal;
- hook policy rejection rehearsal;
- public-priority-fee vs vault bribe path comparison if we add direct
  coinbase payment support.

The current deployed V2 vault example remains at
`../uniswap_v2_trading_vault_deployed.rs`.
