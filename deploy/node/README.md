# Node

Agent operating map for local Ethereum mainnet node deployment files.

## Purpose

- Manage local Reth execution and Lighthouse consensus services.
- Keep deploy scripts and user-systemd units versioned while chain data,
  binaries, logs, and secrets stay outside the repo.
- Provide health and bootstrap commands used by the Rust ETH stack.

## Owns

- User-systemd units under `../systemd/user/`.
- Node install/bootstrap/health scripts under `scripts/`.
- Runtime path conventions sourced from `../../config.env`.

## Does Not Own

- Chain data, JWT secrets, release tarballs, logs, or downloaded binaries.
- Application services beyond their systemd unit definitions.
- Reth/RPC query logic used by Rust crates.

## Data Flow

```text
config.env
  -> deploy/node/scripts/load-config.sh
  -> deploy/systemd/user Reth + Lighthouse user-systemd units
  -> local RPC/WS/IPC/engine endpoints
  -> tx_simulator, reth_chain_query, tx_processor, mempool_processor
```

## Where To Look First

| Need | Start here |
| --- | --- |
| Runtime paths and URLs | `../../config.env` |
| Reth service | `../systemd/user/reth.service` |
| Lighthouse service | `../systemd/user/lighthouse-beacon.service` |
| App service units | `../systemd/user/eth-*.service` |
| Install clients | `scripts/install-latest-clients.sh` |
| Prepare runtime dirs/JWT | `scripts/prepare-dirs.sh` |
| Link systemd units | `scripts/install-user-services.sh` |
| Archive snapshot bootstrap | `scripts/download-reth-archive-snapshot.sh` |
| Health checks | `scripts/healthcheck.sh` |

## Tests And Commands

```bash
cd /home/nima/code/crypto/blockchains/eth/deploy/node
./scripts/prepare-dirs.sh
./scripts/install-latest-clients.sh
ENABLE_NODE_SERVICES=0 ./scripts/install-user-services.sh
./scripts/healthcheck.sh
```

Useful service commands:

```bash
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user status reth.service
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user status lighthouse-beacon.service
XDG_RUNTIME_DIR=/run/user/$(id -u) journalctl --user -u reth.service -f
```

## Current Hazards

- Keep RPC/WS bound to localhost unless there is an explicit exposure plan.
- Stop Reth and Lighthouse before mutating the Reth datadir or downloading an
  archive snapshot into it.
- Do not commit `jwt.hex`, chain data, release tarballs, generated logs, or
  runtime databases.
- `/home/nima/.lighthouse` is a stable bind-mounted datadir path; do not remove
  the mount point.
