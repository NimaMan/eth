// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {IERC20} from "../../../src/interfaces/IERC20.sol";
import {UniswapV4TradingVault} from "../../../src/v4/UniswapV4TradingVault.sol";
import {V4MainnetFixtures} from "../../fixtures/V4MainnetFixtures.sol";
import {TestBase} from "../../utils/TestBase.sol";

interface IUniversalRouterLike {
    function execute(bytes calldata commands, bytes[] calldata inputs, uint256 deadline) external payable;
}

interface IPermit2Like {
    function approve(address token, address spender, uint160 amount, uint48 expiration) external;
}

contract UniswapV4TradingVaultForkGasTest is TestBase {
    struct PoolKey {
        address currency0;
        address currency1;
        uint24 fee;
        int24 tickSpacing;
        address hooks;
    }

    struct ExactInputSingleParams {
        PoolKey poolKey;
        bool zeroForOne;
        uint128 amountIn;
        uint128 amountOutMinimum;
        bytes hookData;
        uint256 minHopPriceX36;
    }

    address private constant OWNER = address(0xA11CE);
    address private constant TREASURY = address(0xBEEF);
    uint256 private constant DEADLINE = 2_000_000_000;
    uint48 private constant PERMIT2_EXPIRATION = type(uint48).max;

    uint8 private constant COMMAND_V4_SWAP = 0x10;
    uint8 private constant ACTION_SWAP_EXACT_IN_SINGLE = 0x06;
    uint8 private constant ACTION_SETTLE = 0x0b;
    uint8 private constant ACTION_TAKE = 0x0e;

    UniswapV4TradingVault private vault;
    uint256 private tokenAmount;

    modifier onlyFork() {
        if (
            V4MainnetFixtures.UNIVERSAL_ROUTER.code.length == 0 || V4MainnetFixtures.PERMIT2.code.length == 0
                || V4MainnetFixtures.USDC.code.length == 0
        ) {
            return;
        }
        _;
    }

    function _setupFork() internal onlyFork {
        vm.pauseGasMetering();
        vm.warp(block.timestamp + 1 hours);
        vm.deal(OWNER, 100 ether);
        vm.deal(address(this), 100 ether);
        vault = new UniswapV4TradingVault(
            OWNER, TREASURY, V4MainnetFixtures.UNIVERSAL_ROUTER, V4MainnetFixtures.PERMIT2, false
        );
    }

    function testGas_Fork_DirectUniversalRouterBuyToEoa() external onlyFork {
        _setupFork();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directBuy(OWNER);
    }

    function testGas_Fork_VaultBuyToVault() external onlyFork {
        _setupFork();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.buyV4ExactEthForTokens{
            value: V4MainnetFixtures.BUY_VALUE
        }(_vaultKey(), V4MainnetFixtures.USDC, V4MainnetFixtures.MIN_USDC_OUT, DEADLINE, "");
    }

    function testGas_Fork_Erc20ApprovePermit2() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForOwner();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        require(IERC20(V4MainnetFixtures.USDC).approve(V4MainnetFixtures.PERMIT2, tokenAmount), "approve failed");
    }

    function testGas_Fork_Permit2ApproveUniversalRouter() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForOwner();
        vm.prank(OWNER);
        require(IERC20(V4MainnetFixtures.USDC).approve(V4MainnetFixtures.PERMIT2, tokenAmount), "approve failed");

        vm.resumeGasMetering();
        vm.prank(OWNER);
        IPermit2Like(V4MainnetFixtures.PERMIT2)
            .approve(
                V4MainnetFixtures.USDC, V4MainnetFixtures.UNIVERSAL_ROUTER, _toUint160(tokenAmount), PERMIT2_EXPIRATION
            );
    }

    function testGas_Fork_DirectUniversalRouterSellPreapproved() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForOwner();
        vm.startPrank(OWNER);
        require(IERC20(V4MainnetFixtures.USDC).approve(V4MainnetFixtures.PERMIT2, tokenAmount), "approve failed");
        IPermit2Like(V4MainnetFixtures.PERMIT2)
            .approve(
                V4MainnetFixtures.USDC, V4MainnetFixtures.UNIVERSAL_ROUTER, _toUint160(tokenAmount), PERMIT2_EXPIRATION
            );
        vm.stopPrank();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directSell(OWNER, _toUint128(tokenAmount), V4MainnetFixtures.MIN_ETH_OUT);
    }

    function testGas_Fork_VaultEmergencySellWithExactPermit2Lifecycle() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForVault();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.emergencySellV4ExactTokensForEth(
            _vaultKey(), V4MainnetFixtures.USDC, _toUint128(tokenAmount), V4MainnetFixtures.MIN_ETH_OUT, DEADLINE, ""
        );
    }

    function _buyUsdcForOwner() internal returns (uint256 received) {
        uint256 beforeBalance = IERC20(V4MainnetFixtures.USDC).balanceOf(OWNER);
        vm.prank(OWNER);
        _directBuy(OWNER);
        received = IERC20(V4MainnetFixtures.USDC).balanceOf(OWNER) - beforeBalance;
    }

    function _buyUsdcForVault() internal returns (uint256 received) {
        vm.prank(OWNER);
        received = vault.buyV4ExactEthForTokens{
            value: V4MainnetFixtures.BUY_VALUE
        }(_vaultKey(), V4MainnetFixtures.USDC, V4MainnetFixtures.MIN_USDC_OUT, DEADLINE, "");
    }

    function _directBuy(address recipient) internal {
        IUniversalRouterLike(V4MainnetFixtures.UNIVERSAL_ROUTER)
        .execute{
            value: V4MainnetFixtures.BUY_VALUE
        }(
            abi.encodePacked(COMMAND_V4_SWAP),
            _inputs(
                _poolKey(),
                true,
                address(0),
                V4MainnetFixtures.USDC,
                V4MainnetFixtures.BUY_VALUE,
                V4MainnetFixtures.MIN_USDC_OUT,
                recipient
            ),
            DEADLINE
        );
    }

    function _directSell(address recipient, uint128 amountIn, uint128 minOut) internal {
        IUniversalRouterLike(V4MainnetFixtures.UNIVERSAL_ROUTER)
            .execute(
                abi.encodePacked(COMMAND_V4_SWAP),
                _inputs(_poolKey(), false, V4MainnetFixtures.USDC, address(0), amountIn, minOut, recipient),
                DEADLINE
            );
    }

    function _inputs(
        PoolKey memory key,
        bool zeroForOne,
        address tokenIn,
        address tokenOut,
        uint128 amountIn,
        uint128 minOut,
        address recipient
    ) private pure returns (bytes[] memory inputs) {
        bytes memory actions = abi.encodePacked(
            bytes1(ACTION_SWAP_EXACT_IN_SINGLE), bytes1(ACTION_SETTLE), bytes1(ACTION_TAKE)
        );
        bytes[] memory params = new bytes[](3);
        params[0] = abi.encode(
            ExactInputSingleParams({
                poolKey: key,
                zeroForOne: zeroForOne,
                amountIn: amountIn,
                amountOutMinimum: minOut,
                hookData: "",
                minHopPriceX36: 0
            })
        );
        params[1] = abi.encode(tokenIn, amountIn, true);
        params[2] = abi.encode(tokenOut, recipient, uint256(0));

        inputs = new bytes[](1);
        inputs[0] = abi.encode(actions, params);
    }

    function _vaultKey() internal pure returns (UniswapV4TradingVault.PoolKey memory) {
        return UniswapV4TradingVault.PoolKey(
            address(0),
            V4MainnetFixtures.USDC,
            V4MainnetFixtures.ETH_USDC_500_FEE,
            V4MainnetFixtures.ETH_USDC_500_TICK_SPACING,
            address(0)
        );
    }

    function _poolKey() private pure returns (PoolKey memory) {
        return PoolKey(
            address(0),
            V4MainnetFixtures.USDC,
            V4MainnetFixtures.ETH_USDC_500_FEE,
            V4MainnetFixtures.ETH_USDC_500_TICK_SPACING,
            address(0)
        );
    }

    function _toUint128(uint256 amount) private pure returns (uint128) {
        require(amount <= type(uint128).max, "uint128");
        // Casting is safe because the range check above enforces uint128 max.
        // forge-lint: disable-next-line(unsafe-typecast)
        return uint128(amount);
    }

    function _toUint160(uint256 amount) private pure returns (uint160) {
        require(amount <= type(uint160).max, "uint160");
        // Casting is safe because the range check above enforces uint160 max.
        // forge-lint: disable-next-line(unsafe-typecast)
        return uint160(amount);
    }
}
