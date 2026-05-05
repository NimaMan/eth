// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusRouter} from "../src/BaygusRouter.sol";
import {
    AdapterConfig,
    CMD_COINBASE_TIP,
    CMD_PERMIT2_TRANSFER_FROM,
    CMD_SWEEP,
    CMD_TRANSFER_FROM,
    CMD_V2_SWAP
} from "../src/types/SharedTypes.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockReentrantReceiver} from "./mocks/MockReentrantReceiver.sol";
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

    function _execute(BaygusRouter router, bytes memory commands, bytes[] memory inputs, uint256 value)
        internal
        returns (bool success, bytes memory data)
    {
        (success, data) = address(router).call{value: value}(abi.encodeCall(BaygusRouter.execute, (commands, inputs)));
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

    function testTransferFromCommandCanPullFromExplicitOwner() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusRouter router = _router(address(0));
        address owner = address(0xA11CE);

        token.mint(owner, 100 ether);
        vm.prank(owner);
        token.approve(address(router), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), owner, 25 ether);

        router.execute(commands, inputs);

        assertEq(token.balanceOf(owner), 75 ether, "owner balance");
        assertEq(token.balanceOf(address(router)), 25 ether, "router balance");
    }

    function testInvalidTransferFromInputReverts() external {
        BaygusRouter router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected invalid transfer input failure");
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

    function testSweepCommandRevertsBelowMinimum() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusRouter router = _router(address(0));

        token.mint(address(router), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), address(this), 11 ether);

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected insufficient sweep balance failure");
    }

    function testSweepCommandTransfersNativeValue() external {
        BaygusRouter router = _router(address(0));
        address recipient = address(0xBEEF);
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0), recipient, 1 ether);

        router.execute{value: 1 ether}(commands, inputs);

        assertEq(recipient.balance, 1 ether, "recipient native balance");
        assertEq(address(router).balance, 0, "router native balance");
    }

    function testNativeSweepRecipientCannotReenterExecute() external {
        BaygusRouter router = _router(address(0));
        MockReentrantReceiver receiver = new MockReentrantReceiver(router);
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0), address(receiver), 1 ether);

        router.execute{value: 1 ether}(commands, inputs);

        assertTrue(receiver.attempted(), "receiver attempted reentry");
        assertFalse(receiver.innerSucceeded(), "inner execute should fail");
        assertEq(address(receiver).balance, 1 ether, "receiver keeps outer sweep");
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

    function testV2SwapCanUseRouterBalanceWhenAmountInIsZero() external {
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
        inputs[1] = abi.encode(uint256(0), uint256(15 ether), path, address(this));

        router.execute(commands, inputs);

        assertEq(tokenIn.balanceOf(address(v2)), 20 ether, "v2 consumed all router input");
        assertEq(tokenIn.balanceOf(address(router)), 0, "router input drained");
        assertEq(tokenOut.balanceOf(address(this)), 15 ether, "output received");
    }

    function testV2SwapRequiresAdapter() external {
        BaygusRouter router = _router(address(0));

        address[] memory path = new address[](2);
        path[0] = address(0x1);
        path[1] = address(0x2);

        bytes memory commands = abi.encodePacked(uint8(CMD_V2_SWAP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1), uint256(1), path, address(this));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected missing V2 adapter failure");
    }

    function testV2SwapRejectsEmptyPath() external {
        MockV2Router v2 = new MockV2Router();
        BaygusRouter router = _router(address(v2));

        address[] memory path = new address[](1);
        path[0] = address(0x1);

        bytes memory commands = abi.encodePacked(uint8(CMD_V2_SWAP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1), uint256(1), path, address(this));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected empty path failure");
    }

    function testCommandLengthMismatchReverts() external {
        BaygusRouter router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP), uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0), address(this), uint256(0));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected command length mismatch failure");
    }

    function testInvalidCommandReverts() external {
        BaygusRouter router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(0xff));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = "";

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected invalid command failure");
    }

    function testPermit2CommandRevertsUntilImplemented() external {
        BaygusRouter router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = "";

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected permit2 adapter missing failure");
    }

    function testCoinbaseTipCommandPaysCurrentCoinbase() external {
        BaygusRouter router = _router(address(0));
        address coinbase = address(0xC011BA5E);
        vm.coinbase(coinbase);
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_COINBASE_TIP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1 ether), block.number, block.number);

        router.execute{value: 1 ether}(commands, inputs);

        assertEq(coinbase.balance, 1 ether, "coinbase tip");
        assertEq(address(router).balance, 0, "router native balance");
    }

    function testCoinbaseTipCommandRejectsWrongBlock() external {
        BaygusRouter router = _router(address(0));
        address coinbase = address(0xC011BA5E);
        vm.coinbase(coinbase);
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_COINBASE_TIP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1 ether), block.number + 1, block.number + 2);

        (bool success,) = _execute(router, commands, inputs, 1 ether);
        assertFalse(success, "expected coinbase block guard failure");
        assertEq(coinbase.balance, 0, "coinbase should not be paid");
    }

    function testCoinbaseTipCommandRejectsMalformedInput() external {
        BaygusRouter router = _router(address(0));
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_COINBASE_TIP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1 ether), uint256(0));

        (bool success,) = _execute(router, commands, inputs, 1 ether);
        assertFalse(success, "expected malformed coinbase tip input failure");
    }
}
