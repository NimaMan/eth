// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {UniswapV2TradingVault} from "../src/UniswapV2TradingVault.sol";
import {MockERC20} from "./mocks/MockERC20.sol";
import {MockFeeOnTransferERC20} from "./mocks/MockFeeOnTransferERC20.sol";
import {MockV2Router} from "./mocks/MockV2Router.sol";
import {TestBase} from "./utils/TestBase.sol";

contract UniswapV2TradingVaultTest is TestBase {
    address private constant OWNER = address(0xA11CE);
    address private constant TREASURY = address(0xBEEF);

    MockERC20 private weth;
    MockERC20 private token;
    MockV2Router private router;
    UniswapV2TradingVault private vault;

    function _deploy() internal {
        weth = new MockERC20("Wrapped Ether", "WETH", 18);
        token = new MockERC20("Token", "TKN", 18);
        router = new MockV2Router(address(weth));
        vault = new UniswapV2TradingVault(OWNER, TREASURY, address(weth), address(router));

        token.mint(address(router), 10_000 ether);
        vm.deal(address(router), 10_000 ether);
        vm.deal(OWNER, 100 ether);
        vm.deal(address(this), 100 ether);
    }

    function testBuyStoresTokenInVaultWithoutApproval() external {
        _deploy();
        router.setNextTokenOut(250 ether);

        vm.prank(OWNER);
        uint256 tokensReceived =
            vault.buyV2ExactEthForTokens{value: 1 ether}(address(token), 200 ether, block.timestamp + 1);

        assertEq(tokensReceived, 250 ether, "tokens received");
        assertEq(token.balanceOf(address(vault)), 250 ether, "vault holds bought token");
        assertEq(token.allowance(address(vault), address(router)), 0, "no sell allowance after buy");
        assertEq(router.lastEthIn(), 1 ether, "router saw buy eth");
    }

    function testEmergencySellApprovesExactAmountAndClearsAllowance() external {
        _deploy();
        router.setNextTokenOut(250 ether);
        router.setNextEthOut(8 ether);

        vm.prank(OWNER);
        vault.buyV2ExactEthForTokens{value: 1 ether}(address(token), 200 ether, block.timestamp + 1);

        assertEq(token.allowance(address(vault), address(router)), 0, "pre-sell allowance");

        vm.prank(OWNER);
        uint256 ethReceived =
            vault.emergencySellV2ExactTokensForEth(address(token), 100 ether, 7 ether, block.timestamp + 1);

        assertEq(ethReceived, 8 ether, "eth received");
        assertEq(TREASURY.balance, 8 ether, "treasury paid");
        assertEq(token.balanceOf(address(vault)), 150 ether, "remaining token");
        assertEq(token.balanceOf(address(router)), 10_000 ether - 250 ether + 100 ether, "router token balance");
        assertEq(token.allowance(address(vault), address(router)), 0, "allowance cleared after sell");
        assertEq(router.lastTokenIn(), 100 ether, "router saw token input");
        assertEq(router.lastRecipient(), address(vault), "router paid vault before treasury transfer");
    }

    function testBuyTracksNetFeeOnTransferTokensReceived() external {
        _deploy();
        MockFeeOnTransferERC20 feeToken = new MockFeeOnTransferERC20("Fee Token", "FEE", 18, 500, address(0xFEE));
        feeToken.mint(address(router), 10_000 ether);
        router.setNextTokenOut(250 ether);

        vm.prank(OWNER);
        uint256 tokensReceived =
            vault.buyV2ExactEthForTokens{value: 1 ether}(address(feeToken), 237 ether, block.timestamp + 1);

        assertEq(tokensReceived, 237.5 ether, "net fee token received");
        assertEq(feeToken.balanceOf(address(vault)), 237.5 ether, "vault stores net token");
        assertEq(feeToken.balanceOf(address(0xFEE)), 12.5 ether, "fee recipient paid");
        assertEq(feeToken.allowance(address(vault), address(router)), 0, "no allowance after buy");
    }

    function testEmergencySellClearsAllowanceForFeeOnTransferToken() external {
        _deploy();
        MockFeeOnTransferERC20 feeToken = new MockFeeOnTransferERC20("Fee Token", "FEE", 18, 500, address(0xFEE));
        feeToken.mint(address(vault), 250 ether);
        router.setNextEthOut(8 ether);

        vm.prank(OWNER);
        uint256 ethReceived =
            vault.emergencySellV2ExactTokensForEth(address(feeToken), 100 ether, 7 ether, block.timestamp + 1);

        assertEq(ethReceived, 8 ether, "eth received");
        assertEq(TREASURY.balance, 8 ether, "treasury paid");
        assertEq(feeToken.balanceOf(address(vault)), 150 ether, "vault spent full amount");
        assertEq(feeToken.balanceOf(address(router)), 95 ether, "router received net tokens");
        assertEq(feeToken.balanceOf(address(0xFEE)), 5 ether, "fee recipient paid");
        assertEq(feeToken.allowance(address(vault), address(router)), 0, "allowance cleared");
    }

    function testOnlyOwnerCanBuyOrSell() external {
        _deploy();
        router.setNextTokenOut(100 ether);

        (bool buySuccess,) = address(vault)
        .call{
            value: 1 ether
        }(abi.encodeCall(UniswapV2TradingVault.buyV2ExactEthForTokens, (address(token), 1, block.timestamp + 1)));
        assertFalse(buySuccess, "non-owner buy should fail");

        vm.prank(OWNER);
        vault.buyV2ExactEthForTokens{value: 1 ether}(address(token), 1, block.timestamp + 1);

        (bool sellSuccess,) = address(vault)
            .call(
                abi.encodeCall(
                    UniswapV2TradingVault.emergencySellV2ExactTokensForEth, (address(token), 1, 1, block.timestamp + 1)
                )
            );
        assertFalse(sellSuccess, "non-owner sell should fail");
    }

    function testBuyRevertsWhenOutputBelowMinimum() external {
        _deploy();
        router.setNextTokenOut(50 ether);

        vm.prank(OWNER);
        (bool success,) = address(vault)
        .call{
            value: 1 ether
        }(
            abi.encodeCall(
                UniswapV2TradingVault.buyV2ExactEthForTokens, (address(token), 100 ether, block.timestamp + 1)
            )
        );

        assertFalse(success, "expected buy slippage failure");
        assertEq(token.balanceOf(address(vault)), 0, "no token retained");
    }

    function testEmergencySellRevertsWhenOutputBelowMinimum() external {
        _deploy();
        router.setNextTokenOut(250 ether);
        router.setNextEthOut(3 ether);

        vm.prank(OWNER);
        vault.buyV2ExactEthForTokens{value: 1 ether}(address(token), 200 ether, block.timestamp + 1);

        vm.prank(OWNER);
        (bool success,) = address(vault)
            .call(
                abi.encodeCall(
                    UniswapV2TradingVault.emergencySellV2ExactTokensForEth,
                    (address(token), 100 ether, 5 ether, block.timestamp + 1)
                )
            );

        assertFalse(success, "expected sell slippage failure");
        assertEq(token.balanceOf(address(vault)), 250 ether, "sell reverted token balance");
        assertEq(token.allowance(address(vault), address(router)), 0, "sell reverted allowance");
        assertEq(TREASURY.balance, 0, "treasury unchanged");
    }

    function testRescueTokenAndEthAreOwnerOnly() external {
        _deploy();
        token.mint(address(vault), 42 ether);
        vm.deal(address(vault), 5 ether);

        address recipient = address(0xCAFE);

        (bool nonOwnerTokenRescue,) =
            address(vault).call(abi.encodeCall(UniswapV2TradingVault.rescueToken, (address(token), recipient, 1 ether)));
        assertFalse(nonOwnerTokenRescue, "non-owner token rescue should fail");

        vm.prank(OWNER);
        vault.rescueToken(address(token), recipient, 10 ether);
        assertEq(token.balanceOf(recipient), 10 ether, "rescued token");

        (bool nonOwnerEthRescue,) =
            address(vault).call(abi.encodeCall(UniswapV2TradingVault.rescueEth, (recipient, 1 ether)));
        assertFalse(nonOwnerEthRescue, "non-owner eth rescue should fail");

        vm.prank(OWNER);
        vault.rescueEth(recipient, 2 ether);
        assertEq(recipient.balance, 2 ether, "rescued eth");
    }
}
