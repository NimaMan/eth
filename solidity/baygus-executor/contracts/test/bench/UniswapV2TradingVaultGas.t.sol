// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {UniswapV2TradingVault} from "../../src/UniswapV2TradingVault.sol";
import {MockERC20} from "../mocks/MockERC20.sol";
import {MockFeeOnTransferERC20} from "../mocks/MockFeeOnTransferERC20.sol";
import {MockV2Router} from "../mocks/MockV2Router.sol";
import {V2GasBenchBase} from "./V2GasBenchBase.sol";

contract UniswapV2TradingVaultGasTest is V2GasBenchBase {
    MockERC20 private weth;
    MockERC20 private token;
    MockFeeOnTransferERC20 private feeToken;
    MockV2Router private router;
    UniswapV2TradingVault private vault;

    function _setupLocal() internal {
        vm.pauseGasMetering();
        vm.warp(1_800_000_000);
        weth = new MockERC20("Wrapped Ether", "WETH", 18);
        token = new MockERC20("Token", "TKN", 18);
        feeToken = new MockFeeOnTransferERC20("Fee Token", "FEE", 18, 500, address(0xFEE));
        router = new MockV2Router(address(weth));
        vault = _deployVault(address(weth), address(router));

        token.mint(address(router), 1_000_000 ether);
        feeToken.mint(address(router), 1_000_000 ether);
        vm.deal(address(router), 1_000_000 ether);
        vm.deal(OWNER, 1_000_000 ether);
        vm.deal(address(this), 1_000_000 ether);
        router.setNextTokenOut(250 ether);
        router.setNextEthOut(8 ether);
    }

    function _seedOwnerTokens(uint256 amount) internal {
        token.mint(OWNER, amount);
    }

    function _seedVaultTokens(uint256 amount) internal {
        token.mint(address(vault), amount);
    }

    function _seedVaultFeeTokens(uint256 amount) internal {
        feeToken.mint(address(vault), amount);
    }

    function testGas_Local_DirectRouterBuyToEoa() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directBuy(address(router), address(weth), address(token), OWNER, BUY_VALUE, 200 ether);
    }

    function testGas_Local_VaultBuyToVault() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.buyV2ExactEthForTokens{value: BUY_VALUE}(address(token), 200 ether, DEADLINE);
    }

    function testGas_Local_StandaloneApprove() external {
        _setupLocal();
        _seedOwnerTokens(TOKEN_AMOUNT);

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _approve(address(token), address(router), TOKEN_AMOUNT);
    }

    function testGas_Local_DirectRouterSellPreapproved() external {
        _setupLocal();
        _seedOwnerTokens(TOKEN_AMOUNT);
        vm.startPrank(OWNER);
        _approve(address(token), address(router), TOKEN_AMOUNT);
        vm.stopPrank();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directSell(address(router), address(token), address(weth), OWNER, TOKEN_AMOUNT, 7 ether);
    }

    function testGas_Local_VaultModeAEmergencySell() external {
        _setupLocal();
        _seedVaultTokens(TOKEN_AMOUNT);

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.emergencySellV2ExactTokensForEth(address(token), TOKEN_AMOUNT, 7 ether, DEADLINE);
    }

    function testGas_Local_FeeOnTransferVaultBuyToVault() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.buyV2ExactEthForTokens{value: BUY_VALUE}(address(feeToken), 237 ether, DEADLINE);
    }

    function testGas_Local_FeeOnTransferVaultModeAEmergencySell() external {
        _setupLocal();
        _seedVaultFeeTokens(TOKEN_AMOUNT);

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.emergencySellV2ExactTokensForEth(address(feeToken), TOKEN_AMOUNT, 7 ether, DEADLINE);
    }
}
