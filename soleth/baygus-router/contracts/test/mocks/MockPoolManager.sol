// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {ILockCallback} from "../../src/interfaces/ILockCallback.sol";
import {PoolKey, SwapParams, BalanceDelta} from "../../src/types/SharedTypes.sol";
import {MockERC20} from "./MockERC20.sol";

contract MockPoolManager {
    address public router;

    BalanceDelta[] private _deltas;
    uint256 private _cursor;

    mapping(address => uint256) public syncedBalance;

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

    SettleCall[] private _settles;
    TakeCall[] private _takes;

    error RouterNotSet();
    error UnauthorizedCaller();

    function setRouter(address router_) external {
        router = router_;
    }

    function sync(address currency) external {
        syncedBalance[currency] = _balanceOf(currency);
    }

    function setNextDelta(int128 amount0, int128 amount1) external {
        delete _deltas;
        _cursor = 0;
        _deltas.push(BalanceDelta({amount0: amount0, amount1: amount1}));
    }

    function queueDelta(int128 amount0, int128 amount1) external {
        _deltas.push(BalanceDelta({amount0: amount0, amount1: amount1}));
    }

    function unlock(bytes calldata data) external returns (bytes memory) {
        if (router == address(0)) revert RouterNotSet();
        if (msg.sender != router) revert UnauthorizedCaller();
        return ILockCallback(router).unlockCallback(data);
    }

    function swap(PoolKey calldata key, SwapParams calldata params, bytes calldata hookData)
        external
        returns (BalanceDelta memory delta)
    {
        if (msg.sender != router) revert UnauthorizedCaller();
        key;
        params;
        hookData;

        if (_cursor < _deltas.length) {
            delta = _deltas[_cursor];
            unchecked {
                ++_cursor;
            }
        }
    }

    function settle(address currency) external payable returns (uint256 amount) {
        if (msg.sender != router) revert UnauthorizedCaller();

        bool isNative = currency == address(0);
        uint256 balance = isNative ? msg.value + syncedBalance[currency] : _balanceOf(currency);
        amount = isNative ? msg.value : balance - syncedBalance[currency];
        syncedBalance[currency] = isNative ? syncedBalance[currency] + amount : balance;
        _settles.push(SettleCall({currency: currency, amount: amount, isNative: isNative}));
    }

    function settleFor(address recipient) external payable returns (uint256 amount) {
        recipient;
        amount = this.settle{value: msg.value}(address(0));
    }

    function take(address currency, address recipient, uint256 amount) external {
        if (msg.sender != router) revert UnauthorizedCaller();
        _takes.push(TakeCall({currency: currency, recipient: recipient, amount: amount}));

        if (currency == address(0)) {
            payable(recipient).transfer(amount);
            syncedBalance[currency] = address(this).balance;
        } else {
            require(MockERC20(currency).transfer(recipient, amount), "take transfer");
            syncedBalance[currency] = MockERC20(currency).balanceOf(address(this));
        }
    }

    function settleHistoryLength() external view returns (uint256) {
        return _settles.length;
    }

    function takeHistoryLength() external view returns (uint256) {
        return _takes.length;
    }

    function getSettleCall(uint256 index) external view returns (SettleCall memory) {
        return _settles[index];
    }

    function getTakeCall(uint256 index) external view returns (TakeCall memory) {
        return _takes[index];
    }

    function _balanceOf(address currency) internal view returns (uint256) {
        if (currency == address(0)) return address(this).balance;
        return MockERC20(currency).balanceOf(address(this));
    }

    receive() external payable {}
}
