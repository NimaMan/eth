# Uniswap V2 Trading Vault Deployment

This folder is the reproducible Ethereum deployment path for
`UniswapV2TradingVault`.

The contract source remains in
`../../solidity/baygus-executor/contracts/src/UniswapV2TradingVault.sol`.
The live tx-prep path remains in `../../alpha/live/trading/`. This folder binds
those pieces into one deployable, auditable on-chain surface.

## Scope

The vault supports Mode A for Uniswap V2-style exits:

- buy ETH to token through the vault and hold the token in the vault;
- keep no standing sell-router allowance after buy;
- approve the exact sell amount and sell in the same emergency-sell
  transaction;
- send ETH proceeds to the configured treasury;
- tolerate fee-on-transfer V2 paths through router methods that support them.

The vault does not contain direct `block.coinbase` payments, private relay
logic, route discovery, quote logic, or gas-rank logic. Miner-bribe policy is
handled off-chain by `alpha/live/trading` and Kartal through the outer
transaction's EIP-1559 fee fields.

## Deployment Flow

1. Fill the non-secret public values in `config/mainnet.toml`.
2. Confirm alpha route policy in `config/alpha-route-policy.mainnet.toml`.
3. Confirm gas-rank and value-cap policy in `config/gas-policy.mainnet.toml`.
4. Confirm Kartal allowlist intent in `config/kartal-policy.mainnet.json`.
5. Run `scripts/00_preflight.sh`.
6. Run `scripts/01_build_and_hash.sh` and copy the outputs into a new
   `runs/<date>-mainnet-vN/` folder.
7. Run `scripts/02_fork_rehearsal.sh` with a fixed fork block.
8. Run `scripts/03_generate_calldata.sh` for representative buy and sell
   requests.
9. Run `scripts/04_kartal_dry_run.sh` against a planner-produced
   `eth_direct_raw_v1` request.
10. Complete `audit/checklist.yaml` and resolve or accept every finding in
    `audit/findings.jsonl`.
11. Run `scripts/05_deploy.sh` only after the signer, owner, treasury, router,
    WETH, gas policy, and Kartal policy are signed off.
12. Run `scripts/06_verify_contract.sh`.
13. Run `scripts/07_post_deploy_smoke.sh`.
14. Store the final receipt, verification, smoke result, and `signoff.json` in
    the run folder.

## Required Mainnet Inputs

| Value | Owner | Notes |
| --- | --- | --- |
| `owner` | Trading ops | Must be the address that can call buy, emergency sell, and rescue methods. |
| `treasury` | Trading ops | Receives ETH from emergency sells. |
| `weth` | Chain config | Mainnet WETH is public chain metadata. |
| `uniswap_v2_router` | Chain config | Mainnet Uniswap V2 router is public chain metadata. |
| `deployment_signer` | Trading ops | Must be distinct from secrets recorded in git. |
| `kartal signer/from` | Kartal ops | Must match the account used in `eth_direct_raw_v1` requests. |
| `gas-rank policy` | Alpha ops | Must choose a ranked candidate; no synthetic value-cap fallback. |

Current temporary test address:

| Role | Address |
| --- | --- |
| owner, treasury, deployer, Kartal `from` | [`0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27`](https://etherscan.io/address/0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27) |

## Secure Deploy Signer

Prefer the encrypted keystore path over `ETH_VAULT_DEPLOYER_PRIVATE_KEY`:

```bash
SIGNER_BACKEND=keystore
ETH_VAULT_DEPLOYER_KEYSTORE=/home/nima/code/crypto/kartal/.state/eth_signer/secrets/eth-signer-keystore.json
ETH_VAULT_DEPLOYER_PASSWORD_FILE=/home/nima/code/crypto/kartal/.state/eth_signer/secrets/eth-signer-password
```

The deploy script records only the keystore/password-file paths in the run
folder. It must never log the decrypted private key.

## Go-live Bottleneck

The contract can be deployed only after the full strategy-to-Kartal live path is
ready to consume its address:

```text
strategy signal
  -> alpha live planner builds UniswapV2TradingVault sell route
  -> final simulation validates exact calldata
  -> gas-rank provider chooses capped priority-fee candidate
  -> prepare_priority_sell builds eth_direct_raw_v1
  -> Kartal validates policy, signs, dry-runs, and broadcasts
  -> alpha reconciles receipt into execution state
```

For a live deployment, the current critical gates are the production planner
input resolver, live pre-submit simulation, live gas-rank provider, Kartal
policy allowlist, and receipt reconciliation.

## Evidence Standard

Every run folder should be enough for a second operator to answer:

- exactly what bytecode was deployed;
- why the constructor arguments were correct;
- what gas cost we expect for buy and emergency sell;
- which Kartal request shape was allowed;
- how the miner bribe was selected and capped;
- whether the deployed contract reads back the expected owner, treasury, WETH,
  and router;
- which human signed off on mainnet broadcast.
