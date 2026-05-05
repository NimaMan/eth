// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../../src/BaygusRouter.sol";
import {CMD_SWEEP} from "../../src/types/SharedTypes.sol";

contract MockReentrantReceiver {
    BaygusRouter public immutable ROUTER;
    bool public attempted;
    bool public innerSucceeded;

    constructor(BaygusRouter router_) {
        ROUTER = router_;
    }

    receive() external payable {
        if (attempted) return;
        attempted = true;

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0), address(this), uint256(0));

        (innerSucceeded,) = address(ROUTER).call(abi.encodeCall(BaygusRouter.execute, (commands, inputs)));
    }
}
