// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {MockERC20} from "./MockERC20.sol";

contract MockPermit2 {
    event TransferFrom(address indexed caller, address indexed from, address indexed to, address token, uint160 amount);

    function transferFrom(address from, address to, uint160 amount, address token) external {
        require(MockERC20(token).transferFrom(from, to, amount), "MockPermit2: transfer");
        emit TransferFrom(msg.sender, from, to, token, amount);
    }
}
