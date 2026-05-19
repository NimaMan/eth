// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {UniswapV4TradingVault} from "../../src/v4/UniswapV4TradingVault.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {TestBase} from "../utils/TestBase.sol";
import {MockPermit2} from "./mocks/MockPermit2.sol";
import {MockUniversalRouterV4} from "./mocks/MockUniversalRouterV4.sol";

contract UniswapV4TradingVaultTest is TestBase {
    address private constant OWNER = address(0xA11CE);
    address private constant TREASURY = address(0xBEEF);

    MockERC20 private token;
    MockPermit2 private permit2;
    MockUniversalRouterV4 private router;
    UniswapV4TradingVault private vault;

    function _deploy(bool hooksAllowed) internal {
        token = new MockERC20("USD Coin", "USDC", 6);
        permit2 = new MockPermit2();
        router = new MockUniversalRouterV4(address(permit2));
        vault = new UniswapV4TradingVault(OWNER, TREASURY, address(router), address(permit2), hooksAllowed);

        token.mint(address(router), 10_000_000e6);
        vm.deal(address(router), 10_000 ether);
        vm.deal(OWNER, 100 ether);
        vm.deal(address(this), 100 ether);
    }

    function testConstructorStoresImmutableConfigAndRejectsZeroAddresses() external {
        _deploy(false);

        assertEq(vault.owner(), OWNER, "owner immutable");
        assertEq(vault.treasury(), TREASURY, "treasury immutable");
        assertEq(vault.universalRouter(), address(router), "router immutable");
        assertEq(vault.permit2(), address(permit2), "permit2 immutable");
        assertFalse(vault.hooksAllowed(), "hooks blocked by default candidate");

        _expectConstructorZeroAddress(address(0), TREASURY, address(router), address(permit2));
        _expectConstructorZeroAddress(OWNER, address(0), address(router), address(permit2));
        _expectConstructorZeroAddress(OWNER, TREASURY, address(0), address(permit2));
        _expectConstructorZeroAddress(OWNER, TREASURY, address(router), address(0));
    }

    function testBuyV4ExactEthForTokensStoresTokenInVault() external {
        _deploy(false);
        router.setNextTokenOut(2_100e6);

        vm.prank(OWNER);
        uint256 received =
            vault.buyV4ExactEthForTokens{value: 1 ether}(_ethUsdcKey(), address(token), 2_000e6, _deadline(), "");

        assertEq(received, 2_100e6, "tokens received");
        assertEq(token.balanceOf(address(vault)), 2_100e6, "vault token balance");
        assertEq(token.allowance(address(vault), address(permit2)), 0, "no erc20 permit2 allowance after buy");
        assertEq(router.lastCaller(), address(vault), "router caller");
        assertEq(router.lastInputCurrency(), address(0), "native input");
        assertEq(router.lastOutputCurrency(), address(token), "token output");
        assertEq(router.lastRecipient(), address(vault), "vault receives token");
        assertEq(router.lastValue(), 1 ether, "router value");
        assertTrue(router.lastZeroForOne(), "ETH to token orientation");
    }

    function testEmergencySellV4ExactTokensForEthApprovesExactlyAndClearsAllowances() external {
        _deploy(false);
        token.mint(address(vault), 2_100e6);
        router.setNextEthOut(0.99 ether);

        vm.prank(OWNER);
        uint256 ethReceived =
            vault.emergencySellV4ExactTokensForEth(_ethUsdcKey(), address(token), 2_000e6, 0.9 ether, _deadline(), "");

        (uint160 permitAmount, uint48 permitExpiration) =
            permit2.allowanceOf(address(vault), address(token), address(router));

        assertEq(ethReceived, 0.99 ether, "eth received");
        assertEq(TREASURY.balance, 0.99 ether, "treasury paid");
        assertEq(token.balanceOf(address(vault)), 100e6, "remaining token");
        assertEq(token.balanceOf(address(router)), 10_000_000e6 + 2_000e6, "router token balance");
        assertEq(token.allowance(address(vault), address(permit2)), 0, "erc20 permit2 allowance cleared");
        assertEq(uint256(permitAmount), 0, "permit2 allowance cleared");
        assertEq(uint256(permitExpiration), 0, "permit2 expiration cleared");
        assertEq(router.lastInputCurrency(), address(token), "token input");
        assertEq(router.lastOutputCurrency(), address(0), "native output");
        assertEq(router.lastRecipient(), address(vault), "vault receives eth before treasury");
        assertFalse(router.lastZeroForOne(), "token to ETH orientation");
    }

    function testHookPolicyRejectsHooksUnlessExplicitlyAllowed() external {
        _deploy(false);
        UniswapV4TradingVault.PoolKey memory hooked = _hookedEthUsdcKey(address(0x1234));

        vm.prank(OWNER);
        (bool blocked, bytes memory blockedReason) = address(vault)
        .call{
            value: 1 ether
        }(abi.encodeCall(UniswapV4TradingVault.buyV4ExactEthForTokens, (hooked, address(token), 1, _deadline(), "")));

        assertFalse(blocked, "hooked pool should be blocked");
        assertEq(
            blockedReason,
            abi.encodeWithSelector(UniswapV4TradingVault.HookNotAllowed.selector, address(0x1234)),
            "hook rejection"
        );

        vault = new UniswapV4TradingVault(OWNER, TREASURY, address(router), address(permit2), true);
        router.setNextTokenOut(2_100e6);

        vm.prank(OWNER);
        uint256 received =
            vault.buyV4ExactEthForTokens{value: 1 ether}(hooked, address(token), 2_000e6, _deadline(), hex"c0ffee");

        assertEq(received, 2_100e6, "hooked buy allowed by explicit constructor policy");
        assertEq(router.lastHooks(), address(0x1234), "hook forwarded");
        assertEq(router.lastHookData(), hex"c0ffee", "hook data forwarded");
    }

    function testHookDataWithoutHookIsRejected() external {
        _deploy(false);

        vm.prank(OWNER);
        (bool success, bytes memory reason) = address(vault)
        .call{
            value: 1 ether
        }(
            abi.encodeCall(
                UniswapV4TradingVault.buyV4ExactEthForTokens,
                (_ethUsdcKey(), address(token), 1, _deadline(), hex"c0ffee")
            )
        );

        assertFalse(success, "hook data without hook should fail");
        assertEq(reason, abi.encodeWithSelector(UniswapV4TradingVault.HookDataWithoutHook.selector), "hook data");
    }

    function testBuyAndSellRejectInvalidInputs() external {
        _deploy(false);
        vm.warp(100);

        (, bytes memory zeroTokenReason) = _callBuy(address(0), 1, block.timestamp + 1, 1 ether);
        assertEq(zeroTokenReason, abi.encodeWithSelector(UniswapV4TradingVault.ZeroAddress.selector), "zero token");

        (, bytes memory zeroValueReason) = _callBuy(address(token), 1, block.timestamp + 1, 0);
        assertEq(zeroValueReason, abi.encodeWithSelector(UniswapV4TradingVault.ZeroAmount.selector), "zero value");

        (, bytes memory expiredBuyReason) = _callBuy(address(token), 1, block.timestamp - 1, 1 ether);
        assertEq(
            expiredBuyReason,
            abi.encodeWithSelector(UniswapV4TradingVault.DeadlineExpired.selector, block.timestamp - 1),
            "expired buy"
        );

        (, bytes memory zeroSellTokenReason) = _callSell(address(0), 1, 1, block.timestamp + 1);
        assertEq(
            zeroSellTokenReason, abi.encodeWithSelector(UniswapV4TradingVault.ZeroAddress.selector), "zero sell token"
        );

        (, bytes memory zeroSellAmountReason) = _callSell(address(token), 0, 1, block.timestamp + 1);
        assertEq(
            zeroSellAmountReason, abi.encodeWithSelector(UniswapV4TradingVault.ZeroAmount.selector), "zero sell amount"
        );

        (, bytes memory expiredSellReason) = _callSell(address(token), 1, 1, block.timestamp - 1);
        assertEq(
            expiredSellReason,
            abi.encodeWithSelector(UniswapV4TradingVault.DeadlineExpired.selector, block.timestamp - 1),
            "expired sell"
        );
    }

    function testInvalidPoolKeyIsRejected() external {
        _deploy(false);
        UniswapV4TradingVault.PoolKey memory invalid =
            UniswapV4TradingVault.PoolKey(address(0), address(0xDEAD), 500, 10, address(0));

        vm.prank(OWNER);
        (bool success, bytes memory reason) = address(vault)
        .call{
            value: 1 ether
        }(abi.encodeCall(UniswapV4TradingVault.buyV4ExactEthForTokens, (invalid, address(token), 1, _deadline(), "")));

        assertFalse(success, "invalid key should fail");
        assertEq(reason, abi.encodeWithSelector(UniswapV4TradingVault.InvalidPoolKey.selector), "invalid key");
    }

    function testOnlyOwnerCanBuySellOrRescue() external {
        _deploy(false);
        token.mint(address(vault), 2_100e6);
        router.setNextTokenOut(2_100e6);
        router.setNextEthOut(0.99 ether);

        (bool buySuccess,) = address(vault)
        .call{
            value: 1 ether
        }(
            abi.encodeCall(
                UniswapV4TradingVault.buyV4ExactEthForTokens, (_ethUsdcKey(), address(token), 1, _deadline(), "")
            )
        );
        assertFalse(buySuccess, "non-owner buy should fail");

        (bool sellSuccess,) = address(vault)
            .call(
                abi.encodeCall(
                    UniswapV4TradingVault.emergencySellV4ExactTokensForEth,
                    (_ethUsdcKey(), address(token), 1, 1, _deadline(), "")
                )
            );
        assertFalse(sellSuccess, "non-owner sell should fail");

        (bool rescueSuccess,) =
            address(vault).call(abi.encodeCall(UniswapV4TradingVault.rescueToken, (address(token), OWNER, 1)));
        assertFalse(rescueSuccess, "non-owner rescue should fail");
    }

    function testSlippageFailuresRevertStateAndAllowance() external {
        _deploy(false);
        router.setNextTokenOut(100e6);

        vm.prank(OWNER);
        (bool buySuccess,) = address(vault)
        .call{
            value: 1 ether
        }(
            abi.encodeCall(
                UniswapV4TradingVault.buyV4ExactEthForTokens, (_ethUsdcKey(), address(token), 200e6, _deadline(), "")
            )
        );
        assertFalse(buySuccess, "buy slippage should fail");
        assertEq(token.balanceOf(address(vault)), 0, "no token retained after failed buy");

        token.mint(address(vault), 2_100e6);
        router.setNextEthOut(0.1 ether);

        vm.prank(OWNER);
        (bool sellSuccess,) = address(vault)
            .call(
                abi.encodeCall(
                    UniswapV4TradingVault.emergencySellV4ExactTokensForEth,
                    (_ethUsdcKey(), address(token), 2_000e6, 0.9 ether, _deadline(), "")
                )
            );
        assertFalse(sellSuccess, "sell slippage should fail");
        assertEq(token.balanceOf(address(vault)), 2_100e6, "token retained after failed sell");
        assertEq(token.allowance(address(vault), address(permit2)), 0, "erc20 allowance reverted");
    }

    function testRescueTokenAndEth() external {
        _deploy(false);
        token.mint(address(vault), 2_100e6);
        vm.deal(address(vault), 5 ether);

        address recipient = address(0xCAFE);

        vm.prank(OWNER);
        vault.rescueToken(address(token), recipient, 100e6);
        assertEq(token.balanceOf(recipient), 100e6, "rescued token");

        vm.prank(OWNER);
        vault.rescueEth(recipient, 2 ether);
        assertEq(recipient.balance, 2 ether, "rescued eth");
    }

    function _expectConstructorZeroAddress(address owner_, address treasury_, address router_, address permit2_)
        internal
    {
        try new UniswapV4TradingVault(owner_, treasury_, router_, permit2_, false) {
            revert AssertionFailed("constructor should reject zero address");
        } catch (bytes memory reason) {
            assertEq(
                reason, abi.encodeWithSelector(UniswapV4TradingVault.ZeroAddress.selector), "constructor zero address"
            );
        }
    }

    function _callBuy(address tokenOut, uint128 minTokensOut, uint256 deadline, uint256 value)
        internal
        returns (bool success, bytes memory reason)
    {
        vm.prank(OWNER);
        return address(vault)
        .call{
            value: value
        }(
            abi.encodeCall(
                UniswapV4TradingVault.buyV4ExactEthForTokens, (_ethUsdcKey(), tokenOut, minTokensOut, deadline, "")
            )
        );
    }

    function _callSell(address tokenIn, uint128 amountIn, uint128 minEthOut, uint256 deadline)
        internal
        returns (bool success, bytes memory reason)
    {
        vm.prank(OWNER);
        return address(vault)
            .call(
                abi.encodeCall(
                    UniswapV4TradingVault.emergencySellV4ExactTokensForEth,
                    (_ethUsdcKey(), tokenIn, amountIn, minEthOut, deadline, "")
                )
            );
    }

    function _ethUsdcKey() internal view returns (UniswapV4TradingVault.PoolKey memory) {
        return UniswapV4TradingVault.PoolKey(address(0), address(token), 500, 10, address(0));
    }

    function _hookedEthUsdcKey(address hook) internal view returns (UniswapV4TradingVault.PoolKey memory) {
        return UniswapV4TradingVault.PoolKey(address(0), address(token), 500, 10, hook);
    }

    function _deadline() internal view returns (uint256) {
        return block.timestamp + 1 hours;
    }
}
