# Ethereum Node (Reth + Lighthouse)

This is the local runbook for the Ethereum node stack on this host:
- Execution layer: Reth (archive/full)
- Consensus layer: Lighthouse (beacon)
- Managed via systemd services

## Repo location
`/home/nima/code/crypto/blockchains/eth/node`

## Components and paths
Execution (Reth):
- Binary: `/home/nima/reth/target/release/reth`
- Systemd unit: `/etc/systemd/system/reth.service`
- Config file referenced by systemd: `/home/nima/reth/reth.toml` (via override)
- Engine API JWT: `/home/nima/lighthouse/jwt.hex`
- Reth logs: `/home/nima/.cache/reth/logs/mainnet/reth.log`

Consensus (Lighthouse):
- Binary: `/home/linuxbrew/.linuxbrew/bin/lighthouse` (brew-managed)
- Systemd unit: `/etc/systemd/system/lighthouse-beacon.service`
- Datadir (default): `/home/nima/.lighthouse`
- Engine API JWT: `/home/nima/lighthouse/jwt.hex`

## Ports
Reth:
- HTTP RPC: `127.0.0.1:8545`
- WS RPC: `127.0.0.1:8546`
- Engine API: `127.0.0.1:8551`

Lighthouse:
- HTTP API: `127.0.0.1:5052`
- P2P: `9020/tcp` and `9020/udp`

## Systemd unit summaries (current)
Reth (`/etc/systemd/system/reth.service`):
```
ExecStart=/home/nima/reth/target/release/reth node \
    --config /home/nima/reth/reth-archive.toml \
    --full \
    --authrpc.jwtsecret /home/nima/lighthouse/jwt.hex \
    --authrpc.addr 127.0.0.1 \
    --authrpc.port 8551 \
    --http \
    --ws \
    --http.addr 127.0.0.1 \
    --http.port 8545 \
    --ws.addr 127.0.0.1 \
    --ws.port 8546 \
    --rpc-max-connections 429496729 \
    --http.api admin,debug,eth,net,trace,txpool,web3,rpc \
    --ws.api admin,debug,eth,net,trace,txpool,web3,rpc
```

Lighthouse (`/etc/systemd/system/lighthouse-beacon.service`):
```
ExecStart=/home/linuxbrew/.linuxbrew/bin/lighthouse bn \
    --network mainnet \
    --execution-endpoint http://localhost:8551 \
    --execution-jwt /home/nima/lighthouse/jwt.hex \
    --checkpoint-sync-url https://mainnet.checkpoint.sigp.io \
    --http \
    --http-address 127.0.0.1 \
    --http-port 5052 \
    --port 9020 \
    --discovery-port 9020
```

## Start / stop
```
# Stop
sudo systemctl stop lighthouse-beacon.service
sudo systemctl stop reth.service

# Start (EL first, then CL)
sudo systemctl start reth.service
sudo systemctl start lighthouse-beacon.service
```

## Status and logs
```
# Service status
systemctl status reth.service
systemctl status lighthouse-beacon.service

# Follow logs
journalctl -u reth.service -f
journalctl -u lighthouse-beacon.service -f
```

## Sync checks
Execution layer:
```
curl -s http://127.0.0.1:8545 -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_syncing","params":[]}'

curl -s http://127.0.0.1:8545 -H 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}'
```

Consensus layer:
```
curl -s http://127.0.0.1:5052/eth/v1/node/syncing
curl -s http://127.0.0.1:5052/eth/v1/beacon/headers/head
```

## Upgrade workflow (summary)
Reth (stable tag):
```
sudo systemctl stop reth.service

git -C /home/nima/reth fetch --tags
# Example: pin to v1.9.3 stable
git -C /home/nima/reth checkout v1.9.3
cargo build --release --manifest-path /home/nima/reth/Cargo.toml

sudo systemctl start reth.service
/home/nima/reth/target/release/reth --version
```

Lighthouse (brew-managed):
```
brew update
brew upgrade lighthouse
/home/linuxbrew/.linuxbrew/bin/lighthouse --version
```

Restart order after upgrades:
```
sudo systemctl restart reth.service
sudo systemctl restart lighthouse-beacon.service
```

## Checkpoint sync (Lighthouse)
Lighthouse refuses insecure genesis sync on mainnet unless you accept weak-subjectivity risk.
Preferred fix is checkpoint sync.

If you need to re-seed the beacon DB, do not delete validator data.
Only remove the beacon DB folder and keep the validators directory intact:
```
# Stop lighthouse first
sudo systemctl stop lighthouse-beacon.service

# Backup only the beacon DB (leave validators/ alone)
mv /home/nima/.lighthouse/mainnet/beacon \
   /home/nima/.lighthouse/mainnet/beacon.bak.$(date +%F)

# Ensure checkpoint sync is set in systemd, then restart
sudo systemctl daemon-reload
sudo systemctl start lighthouse-beacon.service
```

## Incident write-up
See: `/home/nima/code/crypto/blockchains/eth/LIGHTHOUSE_FAILURE_ANALYSIS.md`

## Notes
- Keep RPC endpoints bound to localhost unless you explicitly need remote access.
- Always upgrade both EL and CL clients before scheduled hard forks.
- The JWT secret at `/home/nima/lighthouse/jwt.hex` must be readable by both services.
- The active systemd override uses `/home/nima/reth/reth.toml`. The base unit still references `reth-archive.toml` unless you consolidate the unit files.
