# Systemd

Ethereum execution, beacon, chain-processing, signer, and alpha runtime units.

## Layout

| Path | Purpose |
| --- | --- |
| `*.service`, `*.socket` | System-level units copied to `/etc/systemd/system/`. |
| `user/` | User-level node and alpha units linked into `$HOME/.config/systemd/user`. |

Current system units:

- `reth.service`
- `reth-rpc-kartal-bridge.socket` / `reth-rpc-kartal-bridge.service` - exposes Reth HTTP RPC on the Kartal Docker bridge only.
- `kartal-eth-signer.service` - host-local Unix-socket signer for Kartal ETH tx execution.
- `lighthouse-beacon.service`
- `eth-live-block-processor.service` - Rust `tx_processor` live block processor.
- `eth-live-token-tracker.service` - Python live token tracker and Redis token snapshot publisher.
- `eth-chain-server.service` - Rust in-memory chain/token tracking API and live runtime.
- `eth-mempool-processor.service` - Rust mempool signal detector.

Current user units are documented in `user/README.md` and are installed by
`../node/scripts/install-user-services.sh`.

Build the live processor before starting the service:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo build --release -p tx_processor --bin live_block_processor
```

Build the chain server before starting its service:

```bash
cd /home/nima/code/crypto/blockchains/eth
cargo build --release -p eth_chain_server
```

The service reads `/home/nima/code/crypto/blockchains/eth/config.env`, applies
confirmed blocks through `eth_token`, and serves the live chain/token API from
the in-process `eth_chain_server` runtime.

The token tracker service also reads the shared config, writes logs under
`/home/nima/code/crypto/blockchains/eth/logs`, publishes live token update
notifications on ZMQ port `5557`, and persists token snapshots/indexes in Redis
for the Rust mempool processor.

The mempool processor reads the same config, consumes live block/state snapshots
from Redis, subscribes to token updates on ZMQ port `5557`, and publishes
signals on ZMQ port `5556`. The sidecar `reth_index` database is used for
mempool first-seen arrival analytics when available, but is not required for the
live signal path to run.

If migrating from the old user-level Reth/Lighthouse services, remove the stale
systemd override and stop the user services before starting the system units so
ports `30303`, `8545`, `8546`, `8551`, and `5052` are not double-bound.

## Kartal Reth RPC Bridge

`reth.service` binds HTTP RPC to `127.0.0.1:8545`. Kartal runs inside the
`kartal_default` Docker bridge network and reaches the host at `172.18.0.1`, so
it cannot use localhost directly. The bridge socket exposes only:

```text
172.18.0.1:8545 -> 127.0.0.1:8545
```

Install or refresh it with:

```bash
sudo cp /home/nima/code/crypto/blockchains/eth/deploy/systemd/reth-rpc-kartal-bridge.* /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now reth-rpc-kartal-bridge.socket
```

Verify from the Kartal container:

```bash
docker exec kartal-order-server sh -lc \
  "curl -fsS -H 'Content-Type: application/json' \
  --data '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"eth_blockNumber\",\"params\":[]}' \
  http://172.18.0.1:8545"
```

## Kartal ETH Signer

`kartal-eth-signer.service` runs the local signer daemon on the host and exposes
only `/run/kartal/eth-signer.sock` to Kartal. The key must be loaded by the
signer service, not by the `kartal-order-server` container.

Build and install:

```bash
cd /home/nima/code/crypto/kartal
cargo build --release --bin run_kartal_eth_signer
sudo cp /home/nima/code/crypto/blockchains/eth/deploy/systemd/kartal-eth-signer.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now kartal-eth-signer.service
```

The unit reads non-committed signer settings from:

```text
/etc/kartal/eth-signer.env
```

To import the existing `0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27` key into
an encrypted keystore without putting the private key on a command line:

```bash
cd /home/nima/code/crypto/kartal
cargo run --bin import_kartal_eth_signer_keystore
```

The importer prompts for the private key and keystore password, verifies the
derived address, and writes a gitignored env fragment under
`/home/nima/code/crypto/kartal/.state/eth_signer/secrets/eth-signer.env`.
Copy those non-secret path settings into `/etc/kartal/eth-signer.env` or point
the unit at that env file.

Required live values after the vault is deployed:

```bash
KARTAL_ETH_SIGNER_ADDRESS=0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27
KARTAL_ETH_SIGNER_KEYSTORE_PATH=/run/kartal-secrets/eth-signer-keystore.json
KARTAL_ETH_SIGNER_PASSWORD_FILE=/run/kartal-secrets/eth-signer-password
KARTAL_ETH_SIGNER_ALLOWED_TARGETS=0xDeployedVaultAddress
KARTAL_ETH_SIGNER_ALLOWED_SELECTORS=0x8a62666c,0x5f413d10
KARTAL_ETH_SIGNER_MAX_TRANSACTION_COST_WEI=20000000000000000
```

Then set Kartal order-server to use the socket signer:

```bash
ETH_TX_EXECUTOR_SIGNER_BACKEND=unix_socket
ETH_TX_EXECUTOR_SIGNER_SOCKET_PATH=/run/kartal/eth-signer.sock
ETH_TX_EXECUTOR_SIGNER_ADDRESS=0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27
```
