// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {SafeTransferLib} from "../../../src/libraries/SafeTransferLib.sol";

contract MockPermit2 {
    using SafeTransferLib for address;

    struct PackedAllowance {
        uint160 amount;
        uint48 expiration;
    }

    mapping(address => mapping(address => mapping(address => PackedAllowance))) private _allowance;

    event Approval(
        address indexed owner, address indexed token, address indexed spender, uint160 amount, uint48 expiration
    );

    function approve(address token, address spender, uint160 amount, uint48 expiration) external {
        _allowance[msg.sender][token][spender] = PackedAllowance({amount: amount, expiration: expiration});
        emit Approval(msg.sender, token, spender, amount, expiration);
    }

    function transferFrom(address from, address to, uint160 amount, address token) external {
        PackedAllowance storage allowance_ = _allowance[from][token][msg.sender];
        require(allowance_.amount >= amount, "MockPermit2: allowance");
        require(allowance_.expiration == 0 || allowance_.expiration >= block.timestamp, "MockPermit2: expired");
        if (allowance_.amount != type(uint160).max) {
            allowance_.amount -= amount;
        }
        token.safeTransferFrom(from, to, amount);
    }

    function allowanceOf(address owner, address token, address spender)
        external
        view
        returns (uint160 amount, uint48 expiration)
    {
        PackedAllowance memory allowance_ = _allowance[owner][token][spender];
        return (allowance_.amount, allowance_.expiration);
    }
}
