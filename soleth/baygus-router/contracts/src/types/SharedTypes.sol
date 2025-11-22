// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

struct PoolKey {
    address currency0;
    address currency1;
    uint24 fee;
    int24 tickSpacing;
    address hooks;
}

struct SwapParams {
    bool zeroForOne;
    int256 amountSpecified;
    uint160 sqrtPriceLimitX96;
}

struct BalanceDelta {
    int128 amount0;
    int128 amount1;
}

// Universal Router Commands
uint256 constant CMD_V4_SWAP = 0x01;
uint256 constant CMD_V2_SWAP = 0x02;
uint256 constant CMD_V3_SWAP = 0x03;
// Add more commands as needed