# Systemd

Ethereum execution, beacon, chain-processing, tx-executor, signer, and alpha
runtime units.

## Layout

| Path | Purpose |
| --- | --- |
| `*.service`, `*.socket` | System-level units copied to `/etc/systemd/system/`. |
| `user/` | User-level alpha runner and mempool detector units linked into `$HOME/.config/systemd/user`. |

Current system units:

- `reth.service`
- `lighthouse-beacon.service`
- `eth-chain-server.service` - Rust in-memory chain/token tracking API and live runtime.
- `eth-tx-signer.service` - host-local Unix-socket signer for the ETH tx executor.
- `eth-tx-executor.service` - standalone HTTP service for `/eth/tx/*` submission routes.

Current user units are documented in `user/README.md` and are installed by
`../node/scripts/install-user-services.sh`. Chain-server is not installed as a
user unit; keep exactly one `eth_chain_server` process, owned by the system
`eth-chain-server.service`.

## Chain Server

Build the chain server before starting its service:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo build --release -p eth_chain_server
sudo cp deploy/systemd/eth-chain-server.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now eth-chain-server.service
```

The service reads `/home/nima/code/crypto/blockchains/eth/config.env`, applies
confirmed blocks through `eth_token`, and serves the live chain/token API from
the in-process `eth_chain_server` runtime.

If migrating from old user-level Reth/Lighthouse services, remove the stale
systemd override and stop the user services before starting the system units so
ports `30303`, `8545`, `8546`, `8551`, and `5052` are not double-bound.

## ETH Tx Executor

The ETH tx executor now lives entirely under:

```text
/home/nima/code/crypto/blockchains/eth/tx_executor
```

Build and install the HTTP service plus signer:

```bash
cd /home/nima/code/crypto/blockchains/eth/tx_executor
cargo build --release -p tx_executor_service -p tx_executor_signer
sudo cp /home/nima/code/crypto/blockchains/eth/deploy/systemd/eth-tx-*.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now eth-tx-signer.service
sudo systemctl enable --now eth-tx-executor.service
```

Runtime inputs:

```text
/home/nima/code/crypto/blockchains/eth/config.env
/home/nima/code/crypto/blockchains/eth/config.toml
/etc/eth-tx-executor/signer.env
/etc/eth-tx-executor/executor.env
```

The default HTTP bind is `127.0.0.1:5006`. The default signer socket is:

```text
/run/eth-tx-executor/signer.sock
```

To import the existing `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27` key into
an encrypted keystore without putting the private key on a command line:

```bash
cd /home/nima/code/crypto/blockchains/eth/tx_executor
cargo run -p tx_executor_signer --bin import_eth_tx_signer_keystore
```

The importer prompts for the private key and keystore password, verifies the
derived address, and writes a gitignored env fragment under
`/home/nima/code/crypto/blockchains/eth/tx_executor/.state/signer/secrets/eth-signer.env`.
Copy those non-secret path settings into `/etc/eth-tx-executor/signer.env` or
point the unit at that env file.

Required live signer values after the vault is deployed:

```bash
ETH_TX_SIGNER_ADDRESS=0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27
ETH_TX_SIGNER_KEYSTORE_PATH=/path/to/eth-signer-keystore.json
ETH_TX_SIGNER_PASSWORD_FILE=/path/to/eth-signer-password
ETH_TX_SIGNER_ALLOWED_TARGETS=0xDeployedVaultAddress
ETH_TX_SIGNER_ALLOWED_SELECTORS=0x8a62666c,0x5f413d10
ETH_TX_SIGNER_MAX_TRANSACTION_COST_WEI=20000000000000000
```

Then keep the HTTP executor pointed at the signer:

```bash
ETH_TX_EXECUTOR_SIGNER_BACKEND=unix_socket
ETH_TX_EXECUTOR_SIGNER_SOCKET_PATH=/run/eth-tx-executor/signer.sock
ETH_TX_EXECUTOR_SIGNER_ADDRESS=0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27
```
