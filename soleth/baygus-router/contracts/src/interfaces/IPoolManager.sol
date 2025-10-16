// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {PoolKey, SwapParams, BalanceDelta} from "../types/SharedTypes.sol";

interface IPoolManager {
    function lock(bytes calldata data) external returns (bytes memory);

    function swap(
        PoolKey calldata key,
        SwapParams calldata params,
        bytes calldata hookData
    ) external returns (BalanceDelta memory delta);

    function settle(address currency, uint256 amount) external payable;

    function take(address currency, address recipient, uint256 amount) external;
}
