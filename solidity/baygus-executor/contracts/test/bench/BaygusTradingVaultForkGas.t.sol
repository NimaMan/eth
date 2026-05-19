// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {BaygusTradingVault} from "../../src/BaygusTradingVault.sol";
import {IERC20} from "../../src/interfaces/IERC20.sol";
import {V2MainnetFixtures} from "../fixtures/V2MainnetFixtures.sol";
import {V2GasBenchBase} from "./V2GasBenchBase.sol";

contract BaygusTradingVaultForkGasTest is V2GasBenchBase {
    BaygusTradingVault private vault;
    uint256 private tokenAmount;

    modifier onlyFork() {
        if (V2MainnetFixtures.UNISWAP_V2_ROUTER.code.length == 0) {
            return;
        }
        _;
    }

    function _setupFork() internal onlyFork {
        vm.pauseGasMetering();
        vm.warp(block.timestamp + 1 hours);
        vm.deal(OWNER, 100 ether);
        vm.deal(address(this), 100 ether);
        vault = _deployVault(V2MainnetFixtures.WETH, V2MainnetFixtures.UNISWAP_V2_ROUTER);
    }

    function _buyUsdcForOwner() internal returns (uint256 received) {
        uint256 beforeBalance = IERC20(V2MainnetFixtures.USDC).balanceOf(OWNER);
        vm.prank(OWNER);
        _directBuy(
            V2MainnetFixtures.UNISWAP_V2_ROUTER,
            V2MainnetFixtures.WETH,
            V2MainnetFixtures.USDC,
            OWNER,
            V2MainnetFixtures.BUY_VALUE,
            V2MainnetFixtures.MIN_USDC_OUT
        );
        received = IERC20(V2MainnetFixtures.USDC).balanceOf(OWNER) - beforeBalance;
    }

    function _buyUsdcForVault() internal returns (uint256 received) {
        vm.prank(OWNER);
        received = vault.buyV2ExactEthForTokens{
            value: V2MainnetFixtures.BUY_VALUE
        }(V2MainnetFixtures.USDC, V2MainnetFixtures.MIN_USDC_OUT, DEADLINE);
    }

    function testGas_Fork_DirectRouterBuyToEoa() external onlyFork {
        _setupFork();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directBuy(
            V2MainnetFixtures.UNISWAP_V2_ROUTER,
            V2MainnetFixtures.WETH,
            V2MainnetFixtures.USDC,
            OWNER,
            V2MainnetFixtures.BUY_VALUE,
            V2MainnetFixtures.MIN_USDC_OUT
        );
    }

    function testGas_Fork_VaultBuyToVault() external onlyFork {
        _setupFork();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.buyV2ExactEthForTokens{
            value: V2MainnetFixtures.BUY_VALUE
        }(V2MainnetFixtures.USDC, V2MainnetFixtures.MIN_USDC_OUT, DEADLINE);
    }

    function testGas_Fork_StandaloneApprove() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForOwner();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _approve(V2MainnetFixtures.USDC, V2MainnetFixtures.UNISWAP_V2_ROUTER, tokenAmount);
    }

    function testGas_Fork_DirectRouterSellPreapproved() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForOwner();
        vm.startPrank(OWNER);
        _approve(V2MainnetFixtures.USDC, V2MainnetFixtures.UNISWAP_V2_ROUTER, tokenAmount);
        vm.stopPrank();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directSell(
            V2MainnetFixtures.UNISWAP_V2_ROUTER,
            V2MainnetFixtures.USDC,
            V2MainnetFixtures.WETH,
            OWNER,
            tokenAmount,
            V2MainnetFixtures.MIN_ETH_OUT
        );
    }

    function testGas_Fork_VaultModeAEmergencySell() external onlyFork {
        _setupFork();
        tokenAmount = _buyUsdcForVault();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.emergencySellV2ExactTokensForEth(
            V2MainnetFixtures.USDC, tokenAmount, V2MainnetFixtures.MIN_ETH_OUT, DEADLINE
        );
    }
}
