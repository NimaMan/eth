# Types

## Problem Domain
This directory establishes the shared data structures, constants, and custom types used throughout the Baygus Router ecosystem. It ensures type safety and facilitates consistent data encoding/decoding between the router, tests, and off-chain SDKs.

## Logic
*   **SharedTypes.sol**: Defines critical structs such as `PoolKey` (identifying a V4 pool), `SwapParams` (swap configuration), and `BalanceDelta` (swap result).
*   **Constants**: Exports command flags (e.g., `CMD_V4_SWAP`, `CMD_CURVE_SWAP`) used by the router's dispatch system to identify the operation type.
