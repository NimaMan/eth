// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface ILockCallback {
    function unlockCallback(bytes calldata data) external returns (bytes memory);
}
