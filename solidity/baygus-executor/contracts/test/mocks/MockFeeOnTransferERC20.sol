// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {MockERC20} from "./MockERC20.sol";

contract MockFeeOnTransferERC20 is MockERC20 {
    uint256 public immutable feeBps;
    address public immutable feeRecipient;

    constructor(string memory name_, string memory symbol_, uint8 decimals_, uint256 feeBps_, address feeRecipient_)
        MockERC20(name_, symbol_, decimals_)
    {
        require(feeBps_ <= 1_000, "MockFOT: fee too high");
        require(feeRecipient_ != address(0), "MockFOT: fee recipient");
        feeBps = feeBps_;
        feeRecipient = feeRecipient_;
    }

    function _transfer(address from, address to, uint256 amount) internal override {
        require(balanceOf[from] >= amount, "MockERC20: balance");

        uint256 fee = amount * feeBps / 10_000;
        uint256 net = amount - fee;

        balanceOf[from] -= amount;
        balanceOf[to] += net;
        emit Transfer(from, to, net);

        if (fee > 0) {
            balanceOf[feeRecipient] += fee;
            emit Transfer(from, feeRecipient, fee);
        }
    }
}
