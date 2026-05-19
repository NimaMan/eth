# V4 Trading Vault Simulation Examples

Examples for the V4 vault route rehearsal belong here. These examples are
intended to produce JSON that can be saved under
`onchain-deployments/uniswap-v4-trading-vault/simulations/reports/`.

## Direct Universal Router Baseline

Run the current direct-route baseline:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -q -p tx_simulator \
  --example uniswap_v4_trading_vault_direct_baseline \
  | tee onchain-deployments/uniswap-v4-trading-vault/simulations/reports/eth-usdc-500-no-hook-direct-baseline.json \
  | jq '{fixture_id, block, buy: .buy.transaction.gas_used, sell: .sell.transaction.gas_used, totals}'
```

The default fixture is:

`onchain-deployments/uniswap-v4-trading-vault/simulations/route-fixtures/eth-usdc-500-no-hook.json`

The direct baseline executes:

- native ETH -> USDC through the deployed Uniswap V4 Universal Router;
- ERC20 approval from the caller to Permit2;
- Permit2 allowance to the Universal Router;
- USDC -> native ETH through the deployed Universal Router.

It reports total route gas, allowance setup gas, effective gas price, priority
fee spend, balance deltas, and revert reasons when any step fails.

## Required Before Deployment

- candidate vault route simulation;
- direct-vs-vault gas comparison;
- Permit2 allowance lifecycle rehearsal;
- hook policy rejection rehearsal;
- public-priority-fee vs vault bribe path comparison if we add direct
  coinbase payment support.

The current deployed V2 vault example remains at
`../uniswap_v2_trading_vault_deployed.rs`.
