// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {PoolKey, SwapParams} from "../types/SharedTypes.sol";

interface IHookAdapter {
    function beforeSwap(
        address sender,
        address recipient,
        PoolKey calldata key,
        SwapParams calldata params,
        bytes calldata hookData
    ) external;

    function afterSwap(
        address sender,
        address recipient,
        PoolKey calldata key,
        SwapParams calldata params,
        int128 amount0,
        int128 amount1,
        bytes calldata hookData
    ) external;
}
