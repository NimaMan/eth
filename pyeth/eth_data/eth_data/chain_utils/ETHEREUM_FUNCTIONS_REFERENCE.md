# Ethereum Functions Reference

## Overview
This document provides a comprehensive reference for Ethereum function signatures, covering standard protocols, DeFi operations, and security-critical functions. It serves as a complete guide for transaction analysis and blockchain interaction.

## Table of Contents
1. [ERC Standards](#erc-standards)
2. [DeFi Protocols](#defi-protocols)
3. [Liquidity Management](#liquidity-management)
4. [Token Economics](#token-economics)
5. [Security & Access Control](#security--access-control)
6. [NFT Operations](#nft-operations)
7. [Governance Functions](#governance-functions)
8. [Critical Functions for Security](#critical-functions-for-security)

## ERC Standards

### ERC-20 Token Standard
```python
# Core ERC-20 Functions
"06fdde03": "name()",
"95d89b41": "symbol()",
"313ce567": "decimals()",
"18160ddd": "totalSupply()",
"70a08231": "balanceOf(address)",
"dd62ed3e": "allowance(address,address)",
"095ea7b3": "approve(address,uint256)",
"a9059cbb": "transfer(address,uint256)",
"23b872dd": "transferFrom(address,address,uint256)",

# Extended ERC-20 Functions
"39509351": "increaseAllowance(address,uint256)",
"a457c2d7": "decreaseAllowance(address,uint256)",
"d505accf": "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)",  # EIP-2612
```

### ERC-721 NFT Standard
```python
# Core ERC-721 Functions
"6352211e": "ownerOf(uint256)",
"e985e9c5": "isApprovedForAll(address,address)",
"081812fc": "getApproved(uint256)",
"a22cb465": "setApprovalForAll(address,bool)",
"42842e0e": "safeTransferFrom(address,address,uint256)",
"b88d4fde": "safeTransferFrom(address,address,uint256,bytes)",

# ERC-721 Metadata
"c87b56dd": "tokenURI(uint256)",
"6c0360eb": "baseURI()",
```

### ERC-1155 Multi-Token Standard
```python
# Core ERC-1155 Functions
"00fdd58e": "balanceOf(address,uint256)",
"4e1273f4": "balanceOfBatch(address[],uint256[])",
"f242432a": "safeTransferFrom(address,address,uint256,uint256,bytes)",
"2eb2c2d6": "safeBatchTransferFrom(address,address,uint256[],uint256[],bytes)",
"0e89341c": "uri(uint256)",
```

## DeFi Protocols

### Uniswap V2 Router
```python
# Swap Functions
"38ed1739": "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)",
"8803dbee": "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)",
"7ff36ab5": "swapExactETHForTokens(uint256,address[],address,uint256)",
"4a25d94a": "swapTokensForExactETH(uint256,uint256,address[],address,uint256)",
"18cbafe5": "swapExactTokensForETH(uint256,uint256,address[],address,uint256)",
"fb3bdb41": "swapETHForExactTokens(uint256,address[],address,uint256)",

# Fee-on-Transfer Token Support
"5c11d795": "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)",
"b6f9de95": "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)",
"791ac947": "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)",

# Liquidity Functions
"e8e33700": "addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)",
"f305d719": "addLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
"baa2abde": "removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)",
"02751cec": "removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
"2195995c": "removeLiquidityWithPermit(address,address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)",
"ded9382a": "removeLiquidityETHWithPermit(address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)",
"af2979eb": "removeLiquidityETHSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256)",
"5b0d5984": "removeLiquidityETHWithPermitSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256,bool,uint8,bytes32,bytes32)",
```

### Uniswap V3
```python
# Pool Functions
"f3058399": "initialize(uint160)",
"3850c7bd": "slot0()",  # Returns current price and tick
"32148f67": "increaseObservationCardinalityNext(uint16)",
"128acb08": "swap(address,bool,int256,uint160,bytes)",
"70cf754a": "flash(address,uint256,uint256,bytes)",
"a34123a7": "mint(address,int24,int24,uint128,bytes)",
"0c49ccbe": "burn(int24,int24,uint128)",  # Remove liquidity from pool
"4f1eb3d8": "collect(address,int24,int24,uint128,uint128)",

# Position Manager
"88316456": "mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))",
"219f5d17": "increaseLiquidity((uint256,uint256,uint256,uint256,uint256,uint256))",
"0c49ccbe": "decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))",
"fc6f7865": "collect((uint256,address,uint128,uint128))",
"42966c68": "burn(uint256)",  # Burn NFT position

# Factory
"a1671295": "createPool(address,address,uint24)",
"8a7c195f": "enableFeeAmount(uint24,int24)",
```

### Uniswap V4
```python
# Pool Manager Functions
"44c5c7f8": "initialize(PoolKey,uint160,bytes)",
"0d4f319d": "modifyLiquidity(PoolKey,ModifyLiquidityParams,bytes)",
"e7e9cedc": "swap(PoolKey,SwapParams,bytes)",
"02b18fdb": "donate(PoolKey,uint256,uint256,bytes)",
"b1aca175": "take(Currency,address,uint256)",
"3c2784b3": "settle(Currency)",
```

## Liquidity Management

### Add Liquidity Functions
```python
# Standard additions
"e8e33700": "addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)",
"f305d719": "addLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
"ed995307": "addLiquidity(uint256,uint256)",  # Simplified variant

# Lock liquidity
"8af416f6": "lockLPToken(address,uint256,uint256,address,bool,address)",
"4a970be7": "lockTokens(address,uint256,uint256)",
"c9e10b0e": "extendLockDuration(uint256,uint256)",
```

### Remove Liquidity Functions
```python
# Uniswap style
"baa2abde": "removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)",
"02751cec": "removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)",
"af2979eb": "removeLiquidityETHSupportingFeeOnTransferTokens(address,uint256,uint256,uint256,address,uint256)",

# Custom implementations
"26635815": "removeLiquidity(uint256)",
"3ddd8698": "withdrawLiquidity(uint256)",
"db2e21bc": "emergencyWithdraw()",
"2e1a7d4d": "withdraw(uint256)",
"4782f779": "withdrawETH(uint256)",
"853828b6": "withdrawAll()",
"3ccfd60b": "withdraw()",  # No params
"e9fad8ee": "exit()",
```

## Token Economics

### Tax/Fee Functions
```python
# Fee setters
"8a8c523c": "setFees(uint256,uint256)",  # Also setTradingEnabled(bool)
"9012c4a8": "updateFees(uint256,uint256)",
"7a806d6b": "setTaxes(uint256,uint256)",
"3b124fe7": "setBuyTax(uint256)",
"267822f3": "setSellTax(uint256)",
"5068bb0c": "setTax(uint8,uint256)",
"cfb9cfb5": "updateTaxes(uint8,uint8)",
"69fe0e2d": "setFee(uint256)",
"667f6526": "setTax(uint256)",

# Fee exclusions
"f2cc0c18": "excludeFromFee(address)",
"437823ec": "includeInFee(address)",
"ea2f0b37": "excludeFromFees(address,bool)",
"4ada218b": "isExcludedFromFee(address)",
```

### Supply Control
```python
# Minting
"40c10f19": "mint(address,uint256)",
"a0712d68": "mint(uint256)",
"76c71ca1": "adminMint(address,uint256)",
"484b973c": "ownerMint(address,uint256)",
"68c72796": "batchMint(address[],uint256[])",

# Burning
"42966c68": "burn(uint256)",
"79cc6790": "burnFrom(address,uint256)",
"f5298aca": "burn(address,uint256)",
```

### Transaction Limits
```python
# Max transaction
"715018a6": "setMaxTxAmount(uint256)",  # Also renounceOwnership()
"74010ece": "setMaxtxAmount(uint256)",
"c860795d": "updateMaxtx(uint256)",

# Max wallet
"f8b45b05": "setMaxWalletAmount(uint256)",
"ea1644d5": "setMaxWalletSize(uint256)",
"bb67bab8": "changeMaxWallet(uint256)",

# Combined
"9e7ba283": "setLimits(uint256,uint256)",
"f0b37c04": "removeLimits()",
```

## Security & Access Control

### Ownership Management
```python
# Standard ownership
"f2fde38b": "transferOwnership(address)",
"715018a6": "renounceOwnership()",
"79ba5097": "acceptOwnership()",
"8da5cb5b": "owner()",
"e30c3978": "pendingOwner()",

# Role-based access
"d547741f": "revokeRole(bytes32,address)",
"2f2ff15d": "grantRole(bytes32,address)",
"36568abe": "renounceRole(bytes32,address)",
"91d14854": "hasRole(bytes32,address)",
"248a9ca3": "getRoleAdmin(bytes32)",
```

### Trading Controls
```python
# Pause mechanisms
"8456cb59": "pause()",
"3f4ba83a": "unpause()",
"5c975abb": "paused()",
"16c38b3c": "setPaused(bool)",

# Trading enable/disable
"c9567bf9": "enableTrading()",
"fb201b1d": "enableTrading(uint256)",
"8a8c523c": "setTradingEnabled(bool)",
"e93c53bd": "disableTrading()",
"6ddd1713": "setSwapEnabled(bool)",
```

### Blacklist/Whitelist
```python
# Blacklist management
"f9f92be4": "blacklist(address)",
"42cbb15c": "addToBlacklist(address)",
"6fb4adff": "blacklistAddress(address,bool)",
"b515566a": "setBots(address[])",
"3b13f660": "addBot(address)",
"45aac85b": "removeBot(address)",

# Whitelist
"43d726d6": "addToWhitelist(address)",
"eb3d8dcb": "removeFromWhitelist(address)",
"3af32abf": "isWhitelisted(address)",
```

### Emergency Functions
```python
# Emergency withdrawals
"db2e21bc": "emergencyWithdraw()",
"bd69f0a5": "emergencyStop()",
"6a486a8e": "rescue()",

# Token recovery
"9e281a98": "rescueToken(address,uint256)",
"437874fb": "rescueETH(uint256)",
"8980f11f": "recoverERC20(address,uint256)",
"b2118a8d": "withdraw(address,address,uint256)",
```

## NFT Operations

### Minting
```python
"40c10f19": "mint(address,uint256)",  # Also ERC20
"d3fc9864": "mint(address,string)",  # With URI
"a22cb465": "safeMint(address,uint256)",
"755edd17": "mintNFT(address)",
"efef39a1": "mint(uint256)",  # Payable mints
"c7876ea4": "mintBatch(address,uint256)",
```

### Trading
```python
"095ea7b3": "approve(address,uint256)",
"42842e0e": "safeTransferFrom(address,address,uint256)",
"b88d4fde": "safeTransferFrom(address,address,uint256,bytes)",
"a22cb465": "setApprovalForAll(address,bool)",
```

## Governance Functions

### Proposals
```python
"7d5e81e2": "propose(address[],uint256[],string[],bytes[],string)",
"56781388": "castVote(uint256,uint8)",
"5c19a95c": "delegate(address)",
"c7d8113e": "queue(uint256)",
"fe0d94c1": "execute(uint256)",
"2656227d": "cancel(uint256)",
```

### Timelock
```python
"0e18b681": "acceptAdmin()",
"b1b43ae5": "setPendingAdmin(address)",
"c1a287e2": "delay()",
"7d645fab": "setDelay(uint256)",
```

## Critical Functions for Security

### High Risk Functions (Immediate Action Required)
```python
CRITICAL_FUNCTIONS = {
    # Liquidity removal
    "02751cec": "removeLiquidityETH",
    "baa2abde": "removeLiquidity",
    "af2979eb": "removeLiquidityETHSupportingFeeOnTransferTokens",
    "db2e21bc": "emergencyWithdraw",
    
    # Trading control
    "8456cb59": "pause",
    "e93c53bd": "disableTrading",
    
    # Tax manipulation (check parameters)
    "8a8c523c": "setFees/setTradingEnabled",
    "9012c4a8": "updateFees",
    
    # Supply manipulation
    "40c10f19": "mint",
    "68c72796": "batchMint",
}
```

### Function Selector Collisions
Some selectors are used for multiple functions:
- `0x715018a6`: Both `renounceOwnership()` and `setMaxTxAmount(uint256)`
- `0x8a8c523c`: Both `setFees(uint256,uint256)` and `setTradingEnabled(bool)`
- `0x40c10f19`: Both `mint(address,uint256)` for ERC20 and NFTs

Always verify parameter count and contract context when analyzing transactions.