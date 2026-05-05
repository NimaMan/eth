// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface IPermit2 {
    function transferFrom(address from, address to, uint160 amount, address token) external;
}
