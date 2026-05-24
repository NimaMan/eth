# Simulation Reports

Store generated V4 simulation reports here. Reports are evidence, not config.

Required comparison:

- direct Universal Router path;
- candidate vault path;
- gas and priority-fee spend;
- revert reason if any step fails.

The first report to generate is the direct baseline:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -q -p tx_simulator \
  --example uniswap_v4_trading_vault_direct_baseline \
  > deploy/onchain/uniswap-v4-trading-vault/simulations/reports/eth-usdc-500-no-hook-direct-baseline.json
```

Do not overwrite a report that was used for deployment signoff. Put newer runs
in a dated file or a run-specific folder.

After the candidate vault artifact exists, generate the direct-vs-vault
rehearsal report:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo run -q -p tx_simulator \
  --example uniswap_v4_trading_vault_candidate_rehearsal \
  > deploy/onchain/uniswap-v4-trading-vault/simulations/reports/eth-usdc-500-no-hook-candidate-vault-rehearsal.json
```
