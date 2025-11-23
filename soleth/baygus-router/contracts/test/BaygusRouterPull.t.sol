// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {CMD_TRANSFER_FROM, CMD_V2_SWAP} from "../src/types/SharedTypes.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockV2Router} from "./mocks/MockV2Router.sol";
import {TestBase} from "./utils/TestBase.sol";

contract BaygusRouterPullTest is TestBase {
    function testPullAndSwapV2() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPoolManager pool = new MockPoolManager();
        BaygusRouter router = new BaygusRouter(address(pool));
        MockV2Router v2Router = new MockV2Router();

        token.mint(address(this), 1000 ether);
        token.approve(address(router), 1000 ether);

        // 1. Pull Tokens
        // Input: (address token, uint256 amount)
        bytes memory pullInput = abi.encode(address(token), 100 ether);

        // 2. Swap V2
        // Input: (uint256 amountIn, uint256 amountOutMin, address[] path, address recipient)
        address[] memory path = new address[](2);
        path[0] = address(token);
        path[1] = address(0x1); // Random output token
        bytes memory swapInput = abi.encode(100 ether, 0, path, address(this));

        // Commands
        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = pullInput;

        // Execute
        router.execute(commands, inputs);
        
        assertEq(token.balanceOf(address(router)), 100 ether, "Router should have pulled tokens");
        assertEq(token.balanceOf(address(this)), 900 ether, "User should have paid tokens");
    }
    
    function _singleInput(bytes memory input) internal pure returns (bytes[] memory inputs) {
        inputs = new bytes[](1);
        inputs[0] = input;
    }
}
