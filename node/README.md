# Ethereum Node Deployment

Deploy-only files for the local Ethereum mainnet node live here. Runtime data,
downloaded binaries, logs, and secrets stay outside the repo on the 8 TB SSD.

## Current Profile

- Execution client: Reth
- Consensus client: Lighthouse beacon node
- Node type: full/pruned Reth node, not archive
- Service manager: user systemd
- Runtime root: `/home/nima/storage/samsung8tb/ethereum`
- Shared config: `../config.env`
- Lighthouse datadir: `/home/nima/.lighthouse`
- JWT secret: `/home/nima/storage/samsung8tb/ethereum/jwt/jwt.hex`

The `/home/nima/.lighthouse` path is a bind mount to
`/home/nima/storage/samsung8tb/lighthouse`, so it already lives on the 8 TB SSD.
Do not remove the mount point; keep it as the stable Lighthouse datadir.

## Files

| Path | Purpose |
|------|---------|
| `systemd/user/reth.service` | User-systemd unit for the Reth execution client. |
| `systemd/user/lighthouse-beacon.service` | User-systemd unit for the Lighthouse beacon node. |
| `scripts/install-latest-clients.sh` | Downloads latest GitHub release binaries and verifies SHA-256 digests. |
| `scripts/prepare-dirs.sh` | Creates runtime directories and a shared JWT secret outside the repo. |
| `scripts/install-user-services.sh` | Links the checked-in unit files into `~/.config/systemd/user`. |
| `scripts/healthcheck.sh` | Checks local Reth and Lighthouse RPC/health endpoints. |
| `scripts/load-config.sh` | Sources the repository-level `config.env` for node scripts. |

## Ports

Reth:

- HTTP RPC: `127.0.0.1:8545`
- WebSocket RPC: `127.0.0.1:8546`
- Engine API: `127.0.0.1:8551`
- P2P: `30303/tcp` and `30303/udp`
- Prometheus metrics: `127.0.0.1:9001`

Lighthouse:

- HTTP API: `127.0.0.1:5052`
- Prometheus metrics: `127.0.0.1:5054`
- P2P: `9000/tcp`, `9000/udp`, and QUIC on `9001/udp`

## Bootstrap

From this directory:

```bash
./scripts/prepare-dirs.sh
./scripts/install-latest-clients.sh
./scripts/install-user-services.sh
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user start reth.service
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user start lighthouse-beacon.service
```

The service files are linked from this repo into user systemd, so edits here are
picked up after:

```bash
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user daemon-reload
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user restart reth.service lighthouse-beacon.service
```

## Validation

```bash
./scripts/healthcheck.sh

XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user status reth.service
XDG_RUNTIME_DIR=/run/user/$(id -u) systemctl --user status lighthouse-beacon.service
```

Follow logs:

```bash
XDG_RUNTIME_DIR=/run/user/$(id -u) journalctl --user -u reth.service -f
XDG_RUNTIME_DIR=/run/user/$(id -u) journalctl --user -u lighthouse-beacon.service -f
```

Reth sync state:

```bash
curl -s http://127.0.0.1:8545 \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_syncing","params":[]}'
```

Lighthouse sync state:

```bash
curl -s http://127.0.0.1:5052/eth/v1/node/syncing
```

## Operating Notes

- Keep RPC and WS bound to localhost unless there is an explicit auth and
  network exposure plan.
- Reth defaults to archive mode; this deployment explicitly passes `--full`.
- The old Polygon Bor database was removed to provide SSD headroom for ETH.
- Do not commit `jwt.hex`, chain data, release tarballs, or generated logs here.
