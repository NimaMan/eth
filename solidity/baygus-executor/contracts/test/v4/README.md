# V4 Vault Tests

Tests for `src/v4/UniswapV4TradingVault.sol` belong here. This folder is
separate from V2 tests because V4 has different failure modes around Universal
Router commands, Permit2 state, PoolManager settlement, and hooks.

Required before implementation is deployable:

- owner-only access tests;
- exact calldata and route-policy tests;
- Permit2 allowance lifecycle tests;
- hook allowlist or denylist tests;
- buy, emergency sell, rescue, and revert-path tests;
- fork tests against pinned mainnet block data.
