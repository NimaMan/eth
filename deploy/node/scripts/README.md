# Node Scripts

Operational helpers for the local Reth/Lighthouse runtime.

Run these from `deploy/node/` unless a script says otherwise. The scripts source
`load-config.sh`, which resolves the ETH repo root and reads `config.env` by
default.

| Script | Purpose |
| --- | --- |
| `load-config.sh` | Load repo-local `config.env` values without overriding existing environment variables. |
| `prepare-dirs.sh` | Create node runtime directories and the JWT secret if missing. |
| `install-latest-clients.sh` | Download and install the latest Reth and Lighthouse release binaries. |
| `install-user-services.sh` | Link user-level systemd units from `deploy/systemd/user/`. |
| `download-reth-archive-snapshot.sh` | Start an archive snapshot download through a user systemd transient unit. |
| `healthcheck.sh` | Print client versions and local Reth/Lighthouse health responses. |

Do not write chain data, JWT secrets, release archives, logs, or runtime
databases into this folder.
