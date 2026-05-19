// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

import {UniswapV4TradingVault} from "../../../src/v4/UniswapV4TradingVault.sol";
import {MockERC20} from "../../mocks/MockERC20.sol";
import {MockPermit2} from "../../v4/mocks/MockPermit2.sol";
import {MockUniversalRouterV4} from "../../v4/mocks/MockUniversalRouterV4.sol";
import {V4GasBenchBase} from "./V4GasBenchBase.sol";

contract UniswapV4TradingVaultGasTest is V4GasBenchBase {
    MockERC20 private token;
    MockPermit2 private permit2;
    MockUniversalRouterV4 private router;
    UniswapV4TradingVault private vault;

    function _setupLocal() internal {
        vm.pauseGasMetering();
        vm.warp(1_800_000_000);
        token = new MockERC20("USD Coin", "USDC", 6);
        permit2 = new MockPermit2();
        router = new MockUniversalRouterV4(address(permit2));
        vault = _deployVault(address(router), address(permit2));

        token.mint(address(router), 10_000_000e6);
        token.mint(OWNER, 10_000_000e6);
        token.mint(address(vault), 10_000_000e6);
        vm.deal(address(router), 10_000 ether);
        vm.deal(OWNER, 10_000 ether);
        vm.deal(address(this), 10_000 ether);
        router.setNextTokenOut(2_100e6);
        router.setNextEthOut(0.99 ether);
    }

    function testGas_Local_DirectUniversalRouterBuyToEoa() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directBuy(router, _mockKey(), address(token), OWNER, BUY_VALUE, MIN_TOKEN_OUT);
    }

    function testGas_Local_VaultBuyToVault() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.buyV4ExactEthForTokens{value: BUY_VALUE}(_vaultKey(), address(token), MIN_TOKEN_OUT, DEADLINE, "");
    }

    function testGas_Local_Erc20ApprovePermit2() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _approveErc20ToPermit2(address(token), address(permit2), TOKEN_AMOUNT);
    }

    function testGas_Local_Permit2ApproveUniversalRouter() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _approvePermit2ToRouter(permit2, address(token), address(router), TOKEN_AMOUNT);
    }

    function testGas_Local_DirectUniversalRouterSellPreapproved() external {
        _setupLocal();
        vm.startPrank(OWNER);
        _approveErc20ToPermit2(address(token), address(permit2), TOKEN_AMOUNT);
        _approvePermit2ToRouter(permit2, address(token), address(router), TOKEN_AMOUNT);
        vm.stopPrank();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        _directSell(router, _mockKey(), address(token), OWNER, TOKEN_AMOUNT, MIN_ETH_OUT);
    }

    function testGas_Local_VaultEmergencySellWithExactPermit2Lifecycle() external {
        _setupLocal();

        vm.resumeGasMetering();
        vm.prank(OWNER);
        vault.emergencySellV4ExactTokensForEth(_vaultKey(), address(token), TOKEN_AMOUNT, MIN_ETH_OUT, DEADLINE, "");
    }

    function _vaultKey() internal view returns (UniswapV4TradingVault.PoolKey memory) {
        return UniswapV4TradingVault.PoolKey(address(0), address(token), 500, 10, address(0));
    }

    function _mockKey() internal view returns (MockUniversalRouterV4.PoolKey memory) {
        return MockUniversalRouterV4.PoolKey(address(0), address(token), 500, 10, address(0));
    }
}
