# Route Fixtures

The V2 deployed-vault simulator is parameterized by command-line arguments
rather than fixture files. This folder documents the canonical fixture values
used by `scripts/08_run_simulation_suite.sh`.

| Name | Token | Address | Scenario |
| --- | --- | --- | --- |
| `usdc-buy-then-sell` | USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | Buy with ETH, then emergency sell vault-held output. |
| `direct-usdc-buy-approve-then-sell` | USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | Buy with ETH directly through the router, approve exact output, then direct sell. |
| `otto-wallet-transfer-then-sell` | OTTO | `0x12512a46d971BD035D9466246bc46DAf389E281c` | Transfer wallet-held token into the vault, then emergency sell. |
| `direct-otto-wallet-approve-then-sell` | OTTO | `0x12512a46d971BD035D9466246bc46DAf389E281c` | Approve exact wallet-held amount, then direct sell through the router. |
| `otto-current-state-sell-negative` | OTTO | `0x12512a46d971BD035D9466246bc46DAf389E281c` | Attempt sell without a current vault balance; expected to fail in simulation. |

The deployed vault address is
`0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`.
