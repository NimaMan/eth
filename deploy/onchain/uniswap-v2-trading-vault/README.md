# Uniswap V2 Trading Vault Deployment

This folder is the reproducible Ethereum deployment path for
`UniswapV2TradingVault`.

The contract source remains in
`../../solidity/baygus-executor/contracts/src/UniswapV2TradingVault.sol`.
The live tx-prep path remains in `../../alpha/live/trading/`. This folder binds
those pieces into one deployable, auditable on-chain surface.

## Scope

Mainnet deployed vault:
`0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`.

The vault supports Mode A for Uniswap V2-style exits:

- buy ETH to token through the vault and hold the token in the vault;
- keep no standing sell-router allowance after buy;
- approve the exact sell amount and sell in the same emergency-sell
  transaction;
- send ETH proceeds to the configured treasury;
- tolerate fee-on-transfer V2 paths through router methods that support them.

The vault does not contain direct `block.coinbase` payments, private relay
logic, route discovery, quote logic, or gas-rank logic. Miner-bribe policy is
handled off-chain by `alpha/live/trading` and the ETH tx executor through the
outer transaction's EIP-1559 fee fields.

## Live Trading Sequence

For live trading, the final transaction target is the deployed vault, not the
Uniswap V2 router. The vault then calls the router internally:

```text
strategy signal
  -> alpha planner builds vault calldata
  -> final simulator executes the exact vault calldata against current state
  -> ETH tx executor policy verifies from, target, selector, value, gas, fee, and metadata
  -> ETH tx executor signs and dry-runs or broadcasts the vault transaction
  -> UniswapV2TradingVault calls the Uniswap V2 router internally
  -> alpha reconciles the tx receipt into execution state
```

The production target and selectors are:

| Action | `to` | Selector | Custody effect |
| --- | --- | --- | --- |
| Buy | `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597` | `0x8a62666c` | Bought tokens stay in the vault. |
| Emergency sell | `0x28474cbCd780AeEb3ED1501B68254bEd87cF5597` | `0x5f413d10` | The vault approves the exact token amount, sells, clears allowance by reverting on failure, and sends ETH proceeds to treasury. |

Direct-router simulations remain useful only as a baseline for gas and
functional comparison. They are not the final live readiness gate because they
do not exercise the production custody, approval, treasury, or ETH tx executor
allowlist surface.

The extra gas versus direct router calls is an accepted tradeoff for Mode A.
The vault owns token custody after buy, avoids standing router allowances, and
performs the exact approve-and-sell sequence when a sell is submitted. This is
the behavior we want for live trading because alpha and the ETH tx executor only
need to submit a narrow vault call instead of broad arbitrary router calldata.

## Deployment Flow

1. Fill the non-secret public values in `config/mainnet.toml`.
2. Confirm alpha route policy in `config/alpha-route-policy.mainnet.toml`.
3. Confirm gas-rank and value-cap policy in `config/gas-policy.mainnet.toml`.
4. Confirm ETH tx executor allowlist intent in `config/eth-tx-policy.mainnet.json`
   (legacy filename).
5. Run `scripts/00_preflight.sh`.
6. Run `scripts/01_build_and_hash.sh` and copy the outputs into a new
   `runs/<date>-mainnet-vN/` folder.
7. Run `scripts/02_fork_rehearsal.sh` with a fixed fork block.
8. Run `scripts/03_generate_calldata.sh` for representative buy and sell
   requests.
9. Run `scripts/04_eth_tx_executor_dry_run.sh` against a planner-produced
   `eth_direct_raw_v1` request. The script name is legacy; it targets the ETH tx
   executor service.
10. Complete `audit/checklist.yaml` and resolve or accept every finding in
    `audit/findings.jsonl`.
11. Run `scripts/05_deploy.sh` only after the signer, owner, treasury, router,
    WETH, gas policy, and ETH tx executor policy are signed off.
12. Run `scripts/06_verify_contract.sh`.
13. Run `scripts/07_post_deploy_smoke.sh`.
14. Run `scripts/08_run_simulation_suite.sh` against the deployed vault.
15. Store the final receipt, verification, smoke result, and `signoff.json` in
    the run folder.

## Required Mainnet Inputs

| Value | Owner | Notes |
| --- | --- | --- |
| `owner` | Trading ops | Must be the address that can call buy, emergency sell, and rescue methods. |
| `treasury` | Trading ops | Receives ETH from emergency sells. |
| `weth` | Chain config | Mainnet WETH is public chain metadata. |
| `uniswap_v2_router` | Chain config | Mainnet Uniswap V2 router is public chain metadata. |
| `deployment_signer` | Trading ops | Must be distinct from secrets recorded in git. |
| `executor signer/from` | ETH ops | Must match the account used in `eth_direct_raw_v1` requests. |
| `gas-rank policy` | Alpha ops | Must choose a ranked candidate; no synthetic value-cap fallback. |

Current temporary test address:

| Role | Address |
| --- | --- |
| owner, treasury, deployer, executor `from` | [`0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27`](https://etherscan.io/address/0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27) |

## Secure Deploy Signer

Prefer the encrypted keystore path over `ETH_VAULT_DEPLOYER_PRIVATE_KEY`:

```bash
SIGNER_BACKEND=keystore
ETH_VAULT_DEPLOYER_KEYSTORE=/home/nima/code/crypto/blockchains/eth/tx_executor/.state/signer/secrets/eth-signer-keystore.json
ETH_VAULT_DEPLOYER_PASSWORD_FILE=/home/nima/code/crypto/blockchains/eth/tx_executor/.state/signer/secrets/eth-signer-password
```

The deploy script records only the keystore/password-file paths in the run
folder. It must never log the decrypted private key.

## Go-live Bottleneck

The contract can be deployed only after the full strategy-to-ETH-tx-executor
live path is ready to consume its address:

```text
strategy signal
  -> alpha live planner builds UniswapV2TradingVault sell route
  -> final simulation validates exact calldata
  -> gas-rank provider chooses capped priority-fee candidate
  -> prepare_priority_sell builds eth_direct_raw_v1
  -> ETH tx executor validates policy, signs, dry-runs, and broadcasts
  -> alpha reconciles receipt into execution state
```

For a live deployment, the current critical gates are the production planner
input resolver, live pre-submit simulation, live gas-rank provider, ETH tx
executor policy allowlist, and receipt reconciliation.

## Evidence Standard

Every run folder should be enough for a second operator to answer:

- exactly what bytecode was deployed;
- why the constructor arguments were correct;
- what gas cost we expect for buy and emergency sell;
- which ETH tx executor request shape was allowed;
- how the miner bribe was selected and capped;
- whether the deployed contract reads back the expected owner, treasury, WETH,
  and router;
- which human signed off on mainnet broadcast.
