// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {PoolKey, SwapParams, BalanceDelta} from "../types/SharedTypes.sol";

interface IPoolManager {
    function unlock(bytes calldata data) external returns (bytes memory);

    function swap(
        PoolKey calldata key,
        SwapParams calldata params,
        bytes calldata hookData
    ) external returns (BalanceDelta memory delta);

    function settle(address currency) external payable returns (uint256);

    function settleFor(address recipient) external payable returns (uint256);

    function take(address currency, address recipient, uint256 amount) external;
}
