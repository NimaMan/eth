# Deploy

Operational deployment assets owned by the ETH module.

Keep ETH-specific service definitions here so code, runtime configs, and
systemd units move together. Shared infra or non-ETH application units can stay
under the repository-level `deploy/` tree.

## Layout

| Folder | Purpose |
| --- | --- |
| `systemd/` | ETH execution, beacon, chain-processing, token tracking, mempool, and Kartal signer systemd units. |
