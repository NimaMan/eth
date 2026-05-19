# V4 Vault Tests

Tests for `src/v4/UniswapV4TradingVault.sol` belong here. This folder is
separate from V2 tests because V4 has different failure modes around Universal
Router commands, Permit2 state, PoolManager settlement, and hooks.

Current coverage:

- owner-only access tests;
- immutable constructor policy tests;
- exact V4 route encoding checks through the mock Universal Router;
- Permit2 allowance lifecycle tests for sell paths;
- no-hook default policy and explicit hook-enabled constructor coverage;
- buy, emergency sell, rescue, and revert-path tests.
- fork gas tests against the pinned ETH/USDC V4 route fixture.

Still required before implementation is deployable:

- direct-vs-vault report generation using the real Universal Router fixture;
- review of whether hooks should be allowlisted per address instead of a single
  constructor policy flag.
