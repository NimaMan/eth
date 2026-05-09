// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IHookAdapter} from "../../src/interfaces/IHookAdapter.sol";
import {PoolKey, SwapParams} from "../../src/types/SharedTypes.sol";

contract MockHookAdapter is IHookAdapter {
    bool public beforeCalled;
    bool public afterCalled;
    address public beforeSender;
    address public afterRecipient;
    bytes public lastHookData;
    int128 public afterAmount0;
    int128 public afterAmount1;

    function beforeSwap(
        address sender,
        address recipient,
        PoolKey calldata key,
        SwapParams calldata params,
        bytes calldata hookData
    ) external override {
        beforeCalled = true;
        beforeSender = sender;
        lastHookData = hookData;
        // Silence unused warnings for now
        key;
        params;
        recipient;
    }

    function afterSwap(
        address sender,
        address recipient,
        PoolKey calldata key,
        SwapParams calldata params,
        int128 amount0,
        int128 amount1,
        bytes calldata hookData
    ) external override {
        afterCalled = true;
        afterRecipient = recipient;
        lastHookData = hookData;
        afterAmount0 = amount0;
        afterAmount1 = amount1;
        // Silence unused warnings
        sender;
        key;
        params;
    }
}
