// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {PoolKey, SwapParams} from "../types/SharedTypes.sol";

interface IPoolManager {
    function unlock(bytes calldata data) external returns (bytes memory);

    function swap(PoolKey calldata key, SwapParams calldata params, bytes calldata hookData)
        external
        returns (int256 delta);

    function sync(address currency) external;

    function settle() external payable returns (uint256);

    function settleFor(address recipient) external payable returns (uint256);

    function take(address currency, address recipient, uint256 amount) external;
}
