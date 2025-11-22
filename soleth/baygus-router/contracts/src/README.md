# Source Code

## Problem Domain
This directory contains the core smart contract logic for the Baygus Router. Its primary purpose is to act as a central execution point for token swaps across multiple decentralized exchanges, specifically optimizing for Uniswap V4 while supporting legacy protocols (Uniswap V2/V3, SushiSwap) and other major venues like Curve and Balancer.

## Logic
The core logic is implemented in `BaygusRouter.sol`:
*   **Command Dispatch**: Uses a pattern where an `execute` function accepts a byte-encoded command and inputs, dispatching execution to specific internal handlers (e.g., `_v4Swap`, `_v2Swap`).
*   **Protocol Integration**: Directly interacts with protocol routers or pool managers (e.g., calling `swap` on `PoolManager` for V4, or `exchange` on Curve pools).
*   **Multi-Hop Support**: Supports chaining swaps where the output of one swap becomes the input of the next, managed via `_executeMultiHop`.
*   **Slippage & Settlement**: Enforces minimum output amounts (slippage protection) and manages the settlement of funds to and from the router.
