// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusExecutor} from "../src/BaygusExecutor.sol";
import {
    AdapterConfig,
    CMD_COINBASE_TIP,
    CMD_PERMIT2_SIGNATURE_TRANSFER_FROM,
    CMD_PERMIT2_TRANSFER_FROM,
    CMD_SWEEP,
    CMD_TRANSFER_FROM,
    CMD_V2_PAIR_SWAP,
    CMD_V2_SWAP
} from "../src/types/SharedTypes.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockPermit2} from "./mocks/MockPermit2.sol";
import {MockPoolManager} from "./mocks/MockPoolManager.sol";
import {MockReentrantReceiver} from "./mocks/MockReentrantReceiver.sol";
import {MockV2Pair} from "./mocks/MockV2Pair.sol";
import {MockV2Router} from "./mocks/MockV2Router.sol";
import {TestBase} from "./utils/TestBase.sol";

contract BaygusExecutorCommandsTest is TestBase {
    bytes32 private constant BAYGUS_EXECUTION_TYPEHASH =
        keccak256("BaygusExecution(address executor,address caller,bytes32 commandsHash,bytes32 inputsHash)");
    bytes32 private constant PERMIT2_SIGNATURE_TRANSFER_INPUT_TYPEHASH = keccak256(
        "Permit2SignatureTransferInput(address owner,address token,uint256 permittedAmount,uint256 nonce,uint256 deadline,uint256 requestedAmount)"
    );
    string private constant BAYGUS_EXECUTION_WITNESS_TYPE =
        "BaygusExecution witness)BaygusExecution(address executor,address caller,bytes32 commandsHash,bytes32 inputsHash)TokenPermissions(address token,uint256 amount)";

    function _router(address v2Router) internal returns (BaygusExecutor router) {
        router = _executor(v2Router, address(0));
    }

    function _executor(address v2Router, address permit2) internal returns (BaygusExecutor router) {
        MockPoolManager pool = new MockPoolManager();
        AdapterConfig memory adapters = AdapterConfig({
            uniswapV2Router: v2Router,
            sushiswapRouter: address(0),
            uniswapV3Router: address(0),
            balancerVault: address(0),
            permit2: permit2
        });
        router = new BaygusExecutor(address(pool), adapters);
        pool.setRouter(address(router));
    }

    function _execute(BaygusExecutor router, bytes memory commands, bytes[] memory inputs, uint256 value)
        internal
        returns (bool success, bytes memory data)
    {
        (success, data) = address(router).call{value: value}(abi.encodeCall(BaygusExecutor.execute, (commands, inputs)));
    }

    function _permit2SignatureInput(
        address owner,
        address token,
        uint256 permittedAmount,
        uint256 nonce,
        uint256 deadline,
        uint256 requestedAmount,
        bytes memory signature
    ) internal pure returns (bytes memory) {
        return abi.encode(owner, token, permittedAmount, nonce, deadline, requestedAmount, signature);
    }

    function _validPermit2Signature(
        BaygusExecutor router,
        address caller,
        bytes memory commands,
        bytes[] memory inputs
    ) internal pure returns (bytes memory signature) {
        bytes32 witness = _planWitness(address(router), caller, commands, inputs);
        signature = abi.encodePacked("MockPermit2: valid witness signature", witness, BAYGUS_EXECUTION_WITNESS_TYPE);
    }

    function _planWitness(address router, address caller, bytes memory commands, bytes[] memory inputs)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(
            abi.encode(
                BAYGUS_EXECUTION_TYPEHASH, router, caller, keccak256(commands), _inputsHash(commands, inputs, caller)
            )
        );
    }

    function _inputsHash(bytes memory commands, bytes[] memory inputs, address caller) internal pure returns (bytes32) {
        bytes32[] memory inputHashes = new bytes32[](inputs.length);
        for (uint256 i = 0; i < inputs.length; ++i) {
            if (uint8(commands[i]) == CMD_PERMIT2_SIGNATURE_TRANSFER_FROM) {
                inputHashes[i] = _signatureInputHash(inputs[i], caller);
            } else {
                inputHashes[i] = keccak256(inputs[i]);
            }
        }
        return keccak256(abi.encodePacked(inputHashes));
    }

    function _signatureInputHash(bytes memory input, address caller) internal pure returns (bytes32) {
        (
            address owner,
            address token,
            uint256 permittedAmount,
            uint256 nonce,
            uint256 deadline,
            uint256 requestedAmount,
            bytes memory ignoredSignature
        ) = abi.decode(input, (address, address, uint256, uint256, uint256, uint256, bytes));

        ignoredSignature;
        if (owner == address(0)) owner = caller;
        return keccak256(
            abi.encode(
                PERMIT2_SIGNATURE_TRANSFER_INPUT_TYPEHASH,
                owner,
                token,
                permittedAmount,
                nonce,
                deadline,
                requestedAmount
            )
        );
    }

    function testTransferFromCommandPullsTokens() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusExecutor router = _router(address(0));

        token.mint(address(this), 100 ether);
        token.approve(address(router), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), 40 ether);

        router.execute(commands, inputs);

        assertEq(token.balanceOf(address(this)), 60 ether, "payer balance");
        assertEq(token.balanceOf(address(router)), 40 ether, "router balance");
    }

    function testTransferFromCommandRejectsExplicitOwnerInput() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusExecutor router = _router(address(0));
        address owner = address(0xA11CE);

        token.mint(owner, 100 ether);
        vm.prank(owner);
        token.approve(address(router), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), owner, 25 ether);

        (bool success,) = _execute(router, commands, inputs, 0);

        assertFalse(success, "expected explicit owner transfer input failure");
        assertEq(token.balanceOf(owner), 100 ether, "owner balance");
        assertEq(token.balanceOf(address(router)), 0, "router balance");
    }

    function testInvalidTransferFromInputReverts() external {
        BaygusExecutor router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected invalid transfer input failure");
    }

    function testSweepCommandTransfersRouterBalance() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        BaygusExecutor router = _router(address(0));
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
        BaygusExecutor router = _router(address(0));

        token.mint(address(router), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), address(this), 11 ether);

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected insufficient sweep balance failure");
    }

    function testSweepCommandTransfersNativeValue() external {
        BaygusExecutor router = _router(address(0));
        address recipient = address(0xBEEF);
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0), recipient, 1 ether);

        router.execute{value: 1 ether}(commands, inputs);

        assertEq(recipient.balance, 1 ether, "recipient native balance");
        assertEq(address(router).balance, 0, "executor native balance");
    }

    function testNativeSweepRecipientCannotReenterExecute() external {
        BaygusExecutor router = _router(address(0));
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
        BaygusExecutor router = _router(address(v2));

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
        BaygusExecutor router = _router(address(v2));

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

    function testTransferThenV2PairSwapCommand() external {
        MockERC20 tokenIn = new MockERC20("Token In", "TIN", 18);
        MockERC20 tokenOut = new MockERC20("Token Out", "TOUT", 18);
        MockV2Pair pair = new MockV2Pair(address(tokenOut), address(tokenIn));
        BaygusExecutor router = _router(address(0));

        tokenIn.mint(address(this), 100 ether);
        tokenOut.mint(address(pair), 25 ether);
        tokenIn.approve(address(router), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM), uint8(CMD_V2_PAIR_SWAP));
        bytes[] memory inputs = new bytes[](2);
        inputs[0] = abi.encode(address(tokenIn), 20 ether);
        inputs[1] = abi.encode(
            address(pair), address(tokenIn), uint256(20 ether), uint256(15 ether), uint256(0), address(this)
        );

        router.execute(commands, inputs);

        assertEq(tokenIn.balanceOf(address(this)), 80 ether, "input spent");
        assertEq(tokenIn.balanceOf(address(pair)), 20 ether, "pair received input");
        assertEq(tokenOut.balanceOf(address(this)), 15 ether, "output received");
        assertEq(tokenIn.allowance(address(router), address(pair)), 0, "pair approval not used");
    }

    function testV2PairSwapCanUseRouterBalanceWhenAmountInIsZero() external {
        MockERC20 tokenIn = new MockERC20("Token In", "TIN", 18);
        MockERC20 tokenOut = new MockERC20("Token Out", "TOUT", 18);
        MockV2Pair pair = new MockV2Pair(address(tokenIn), address(tokenOut));
        BaygusExecutor router = _router(address(0));

        tokenIn.mint(address(this), 100 ether);
        tokenOut.mint(address(pair), 25 ether);
        tokenIn.approve(address(router), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_TRANSFER_FROM), uint8(CMD_V2_PAIR_SWAP));
        bytes[] memory inputs = new bytes[](2);
        inputs[0] = abi.encode(address(tokenIn), 20 ether);
        inputs[1] =
            abi.encode(address(pair), address(tokenIn), uint256(0), uint256(0), uint256(15 ether), address(this));

        router.execute(commands, inputs);

        assertEq(tokenIn.balanceOf(address(pair)), 20 ether, "pair received all router input");
        assertEq(tokenIn.balanceOf(address(router)), 0, "router input drained");
        assertEq(tokenOut.balanceOf(address(this)), 15 ether, "output received");
    }

    function testV2SwapRequiresAdapter() external {
        BaygusExecutor router = _router(address(0));

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
        BaygusExecutor router = _router(address(v2));

        address[] memory path = new address[](1);
        path[0] = address(0x1);

        bytes memory commands = abi.encodePacked(uint8(CMD_V2_SWAP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1), uint256(1), path, address(this));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected empty path failure");
    }

    function testCommandLengthMismatchReverts() external {
        BaygusExecutor router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_SWEEP), uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0), address(this), uint256(0));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected command length mismatch failure");
    }

    function testInvalidCommandReverts() external {
        BaygusExecutor router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(0xff));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = "";

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected invalid command failure");
    }

    function testPermit2TransferFromCommandPullsTokens() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        token.mint(address(this), 100 ether);
        token.approve(address(permit2), 100 ether);
        permit2.approve(address(token), address(router), 100 ether, type(uint48).max);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), 40 ether);

        router.execute(commands, inputs);

        assertEq(token.balanceOf(address(this)), 60 ether, "payer balance");
        assertEq(token.balanceOf(address(router)), 40 ether, "executor balance");
    }

    function testPermit2TransferFromCommandRejectsExplicitOwnerInput() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));
        address owner = address(0xA11CE);

        token.mint(owner, 100 ether);
        vm.prank(owner);
        token.approve(address(permit2), 100 ether);
        vm.prank(owner);
        permit2.approve(address(token), address(router), 100 ether, type(uint48).max);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), owner, 25 ether);

        (bool success,) = _execute(router, commands, inputs, 0);

        assertFalse(success, "expected explicit owner Permit2 input failure");
        assertEq(token.balanceOf(owner), 100 ether, "owner balance");
        assertEq(token.balanceOf(address(router)), 0, "executor balance");
    }

    function testPermit2CommandRequiresAdapter() external {
        BaygusExecutor router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(0x1), uint256(1));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected permit2 adapter missing failure");
    }

    function testPermit2CommandRejectsMalformedInput() external {
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected invalid permit2 transfer input failure");
    }

    function testPermit2CommandRejectsAmountOverflow() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(address(token), uint256(type(uint160).max) + 1);

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected permit2 amount overflow failure");
    }

    function testPermit2SignatureTransferCommandPullsTokens() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        token.mint(address(this), 100 ether);
        token.approve(address(permit2), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] =
            _permit2SignatureInput(address(0), address(token), 40 ether, 7, block.timestamp + 1 days, 40 ether, "");
        inputs[0] = _permit2SignatureInput(
            address(0),
            address(token),
            40 ether,
            7,
            block.timestamp + 1 days,
            40 ether,
            _validPermit2Signature(router, address(this), commands, inputs)
        );

        router.execute(commands, inputs);

        assertEq(token.balanceOf(address(this)), 60 ether, "payer balance");
        assertEq(token.balanceOf(address(router)), 40 ether, "executor balance");
    }

    function testPermit2SignatureTransferCommandCanPullFromExplicitOwner() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));
        address owner = address(0xA11CE);

        token.mint(owner, 100 ether);
        vm.prank(owner);
        token.approve(address(permit2), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = _permit2SignatureInput(owner, address(token), 50 ether, 8, block.timestamp + 1 days, 25 ether, "");
        inputs[0] = _permit2SignatureInput(
            owner,
            address(token),
            50 ether,
            8,
            block.timestamp + 1 days,
            25 ether,
            _validPermit2Signature(router, address(this), commands, inputs)
        );

        router.execute(commands, inputs);

        assertEq(token.balanceOf(owner), 75 ether, "owner balance");
        assertEq(token.balanceOf(address(router)), 25 ether, "executor balance");
    }

    function testPermit2SignatureTransferCommandRejectsReplayedNonce() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        token.mint(address(this), 100 ether);
        token.approve(address(permit2), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] =
            _permit2SignatureInput(address(0), address(token), 40 ether, 9, block.timestamp + 1 days, 20 ether, "");
        inputs[0] = _permit2SignatureInput(
            address(0),
            address(token),
            40 ether,
            9,
            block.timestamp + 1 days,
            20 ether,
            _validPermit2Signature(router, address(this), commands, inputs)
        );

        router.execute(commands, inputs);
        (bool success,) = _execute(router, commands, inputs, 0);

        assertFalse(success, "expected replayed Permit2 nonce failure");
        assertEq(token.balanceOf(address(this)), 80 ether, "only first transfer spent");
        assertEq(token.balanceOf(address(router)), 20 ether, "executor first transfer");
    }

    function testPermit2SignatureTransferCommandRejectsMutatedPlan() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));
        address attacker = address(0xBEEF);

        token.mint(address(this), 100 ether);
        token.approve(address(permit2), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM), uint8(CMD_SWEEP));
        bytes[] memory inputs = new bytes[](2);
        inputs[0] =
            _permit2SignatureInput(address(0), address(token), 40 ether, 11, block.timestamp + 1 days, 40 ether, "");
        inputs[1] = abi.encode(address(token), address(this), uint256(40 ether));
        inputs[0] = _permit2SignatureInput(
            address(0),
            address(token),
            40 ether,
            11,
            block.timestamp + 1 days,
            40 ether,
            _validPermit2Signature(router, address(this), commands, inputs)
        );
        inputs[1] = abi.encode(address(token), attacker, uint256(40 ether));

        (bool success,) = _execute(router, commands, inputs, 0);

        assertFalse(success, "expected mutated plan witness failure");
        assertEq(token.balanceOf(address(this)), 100 ether, "payer balance");
        assertEq(token.balanceOf(address(router)), 0, "executor balance");
        assertEq(token.balanceOf(attacker), 0, "attacker balance");
    }

    function testPermit2SignatureTransferCommandRejectsWrongCaller() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));
        address attacker = address(0xBEEF);

        token.mint(address(this), 100 ether);
        token.approve(address(permit2), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] =
            _permit2SignatureInput(address(this), address(token), 40 ether, 12, block.timestamp + 1 days, 40 ether, "");
        inputs[0] = _permit2SignatureInput(
            address(this),
            address(token),
            40 ether,
            12,
            block.timestamp + 1 days,
            40 ether,
            _validPermit2Signature(router, address(this), commands, inputs)
        );

        vm.prank(attacker);
        (bool success,) = _execute(router, commands, inputs, 0);

        assertFalse(success, "expected wrong caller witness failure");
        assertEq(token.balanceOf(address(this)), 100 ether, "owner balance");
        assertEq(token.balanceOf(address(router)), 0, "executor balance");
    }

    function testPermit2SignatureTransferCommandRejectsBadSignature() external {
        MockERC20 token = new MockERC20("Token", "TKN", 18);
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        token.mint(address(this), 100 ether);
        token.approve(address(permit2), 100 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = _permit2SignatureInput(
            address(0), address(token), 40 ether, 10, block.timestamp + 1 days, 40 ether, bytes("bad")
        );

        (bool success,) = _execute(router, commands, inputs, 0);

        assertFalse(success, "expected bad Permit2 signature failure");
        assertEq(token.balanceOf(address(router)), 0, "executor balance");
    }

    function testPermit2SignatureTransferCommandRequiresAdapter() external {
        BaygusExecutor router = _router(address(0));

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = _permit2SignatureInput(address(0), address(0x1), 1, 1, block.timestamp + 1, 1, "");

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected permit2 adapter missing failure");
    }

    function testPermit2SignatureTransferCommandRejectsMalformedInput() external {
        MockPermit2 permit2 = new MockPermit2();
        BaygusExecutor router = _executor(address(0), address(permit2));

        bytes memory commands = abi.encodePacked(uint8(CMD_PERMIT2_SIGNATURE_TRANSFER_FROM));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1));

        (bool success,) = _execute(router, commands, inputs, 0);
        assertFalse(success, "expected invalid permit2 signature transfer input failure");
    }

    function testCoinbaseTipCommandPaysCurrentCoinbase() external {
        BaygusExecutor router = _router(address(0));
        address coinbase = address(0xC011BA5E);
        vm.coinbase(coinbase);
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_COINBASE_TIP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1 ether), block.number, block.number);

        router.execute{value: 1 ether}(commands, inputs);

        assertEq(coinbase.balance, 1 ether, "coinbase tip");
        assertEq(address(router).balance, 0, "executor native balance");
    }

    function testCoinbaseTipCommandRejectsWrongBlock() external {
        BaygusExecutor router = _router(address(0));
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
        BaygusExecutor router = _router(address(0));
        vm.deal(address(this), 10 ether);

        bytes memory commands = abi.encodePacked(uint8(CMD_COINBASE_TIP));
        bytes[] memory inputs = new bytes[](1);
        inputs[0] = abi.encode(uint256(1 ether), uint256(0));

        (bool success,) = _execute(router, commands, inputs, 1 ether);
        assertFalse(success, "expected malformed coinbase tip input failure");
    }
}
