// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ILockCallback} from "../../src/interfaces/ILockCallback.sol";
import {PoolKey, SwapParams, BalanceDelta} from "../../src/types/SharedTypes.sol";
import {MockERC20} from "./MockERC20.sol";

contract MockPoolManager {
    address public router;

    BalanceDelta[] private _deltaQueue;
    uint256 private _deltaCursor;

    address private _syncedCurrency;
    uint256 private _syncedReserves;

    struct SettleCall {
        address currency;
        uint256 amount;
        bool isNative;
    }

    struct TakeCall {
        address currency;
        address recipient;
        uint256 amount;
    }

    SettleCall[] private _settleHistory;
    TakeCall[] private _takeHistory;

    error RouterNotSet();
    error UnauthorizedCaller();

    function setRouter(address router_) external {
        router = router_;
    }

    function setNextDelta(int128 amount0, int128 amount1) external {
        delete _deltaQueue;
        _deltaCursor = 0;
        _deltaQueue.push(BalanceDelta({amount0: amount0, amount1: amount1}));
    }

    function queueDelta(int128 amount0, int128 amount1) external {
        _deltaQueue.push(BalanceDelta({amount0: amount0, amount1: amount1}));
    }

    function unlock(bytes calldata data) external returns (bytes memory) {
        if (router == address(0)) revert RouterNotSet();
        if (msg.sender != router) revert UnauthorizedCaller();
        return ILockCallback(router).unlockCallback(data);
    }

    function swap(
        PoolKey calldata key,
        SwapParams calldata params,
        bytes calldata data
    ) external returns (BalanceDelta memory delta) {
        if (msg.sender != router) revert UnauthorizedCaller();
        key;
        params;
        data;
        if (_deltaCursor < _deltaQueue.length) {
            delta = _deltaQueue[_deltaCursor];
            _deltaCursor++;
        } else if (_deltaQueue.length != 0) {
            delta = _deltaQueue[_deltaQueue.length - 1];
        }
    }

    function settle(address currency) external payable returns (uint256) {
        if (msg.sender != router) revert UnauthorizedCaller();
        uint256 amount;
        bool isNative;
        if (currency == address(0)) {
            amount = msg.value;
            isNative = true;
        } else {
            uint256 balance = _balanceOf(currency);
            amount = balance - _syncedReserves;
            isNative = false;
        }

        _settleHistory.push(SettleCall({currency: currency, amount: amount, isNative: isNative}));
        _syncedCurrency = address(0);
        _syncedReserves = 0;

        return amount;
    }

    function settleFor(address recipient) external payable returns (uint256) {
        recipient;
        return this.settle{value: msg.value}(address(0));
    }

    function take(address currency, address recipient, uint256 amount) external {
        if (msg.sender != router) revert UnauthorizedCaller();
        _takeHistory.push(TakeCall({currency: currency, recipient: recipient, amount: amount}));

        if (amount == 0) return;

        if (currency == address(0)) {
            payable(recipient).transfer(amount);
        } else {
            MockERC20(currency).transfer(recipient, amount);
        }
    }

    function settleHistoryLength() external view returns (uint256) {
        return _settleHistory.length;
    }

    function takeHistoryLength() external view returns (uint256) {
        return _takeHistory.length;
    }

    function getSettleCall(uint256 index) external view returns (SettleCall memory) {
        return _settleHistory[index];
    }

    function getTakeCall(uint256 index) external view returns (TakeCall memory) {
        return _takeHistory[index];
    }

    function _balanceOf(address currency) internal view returns (uint256) {
        if (currency == address(0)) {
            return address(this).balance;
        }
        return MockERC20(currency).balanceOf(address(this));
    }

    receive() external payable {}
}
