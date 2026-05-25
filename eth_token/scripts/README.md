# ETH Token Scripts

Small operational and audit scripts that belong to `eth_token` live here. These
scripts may call local HTTP APIs or RPC endpoints, but the facts they check are
token/pool tracking facts owned by `eth_token`.

Promote stable checks into Rust tests or crate code when they become part of the
production contract.
