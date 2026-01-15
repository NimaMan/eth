# Mocks

## Problem Domain
This directory provides simulated implementations of external contracts. These mocks are essential for unit testing the Baygus Router in isolation, allowing developers to verify router logic without relying on the actual complex implementations of protocols like Uniswap V4 or requiring a mainnet fork for every test.

## Logic
*   **MockPoolManager**: Simulates the behavior of the Uniswap V4 PoolManager, including `unlock`, `swap`, `settle`, and `take` functions, to test the router's callback and settlement flow.
*   **MockERC20**: A simplified ERC20 token for testing transfers and approvals.
*   **MockHookAdapter**: A test implementation of the `IHookAdapter` to verify that the router correctly triggers `beforeSwap` and `afterSwap` hooks.
