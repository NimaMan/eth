// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {AdapterConfig, CMD_SWEEP, CMD_TRANSFER_FROM, CMD_V2_SWAP} from "../src/types/SharedTypes.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockV2Router} from "./mocks/MockV2Router.sol";
import {TestBase} from "./utils/TestBase.sol";

contract BaygusRouterCommandsTest is TestBase {
    function _router(address v2Router) internal returns (BaygusRouter router) {
        MockPoolManager pool = new MockPoolManager();
        AdapterConfig memory adapters = AdapterConfig({
            uniswapV2Router: v2Router,
            sushiswapRouter: address(0),
            uniswapV3Router: address(0),
            balancerVault: address(0),
            permit2: address(0)
        });
        router = new BaygusRouter(address(pool), adapters);
        pool.setRouter(address(router));
    }

    function testTransferFromCommandPullsTokens() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusRouter router = _router(address(0));

        token.mint(address(this), 100 ether);
        token.approve(address(router), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), 40 ether);

        router.execute(commands, inputs);

        assertEq(token.balanceOf(address(this)), 60 ether, "payer balance");
        assertEq(token.balanceOf(address(router)), 40 ether, "router balance");
    }

    function testSweepCommandTransfersRouterBalance() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusRouter router = _router(address(0));
        address recipient = address(0xBEEF);

        token.mint(address(router), 75 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), recipient, 75 ether);

        router.execute(commands, inputs);

        assertEq(token.balanceOf(address(router)), 0, "router emptied");
        assertEq(token.balanceOf(recipient), 75 ether, "recipient paid");
    }

    function testTransferThenV2SwapCommand() external {
        MockERC20 tokenIn = new MockERC20("Token In", "TIN", 18);
        MockERC20 tokenOut = new MockERC20("Token Out", "TOUT", 18);
        MockV2Router v2 = new MockV2Router();
        BaygusRouter router = _router(address(v2));

        tokenIn.mint(address(this), 100 ether);
        tokenOut.mint(address(v2), 25 ether);
        tokenIn.approve(address(router), 100 ether);

        address[] memory path = new address[](2);
        path[0] = address(tokenIn);
        path[1] = address(tokenOut);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM), uint8(CMD_V2_SWAP));
        bytes[] memory inputs = new bytes[](2);
        inputs[0] = abi.encode(address(tokenIn), 20 ether);
        inputs[1] = abi.encode(uint256(20 ether), uint256(15 ether), path, address(this));

        router.execute(commands, inputs);

        assertEq(tokenIn.balanceOf(address(this)), 80 ether, "input spent");
        assertEq(tokenIn.balanceOf(address(v2)), 20 ether, "v2 consumed input");
        assertEq(tokenOut.balanceOf(address(this)), 15 ether, "output received");
    }
}
