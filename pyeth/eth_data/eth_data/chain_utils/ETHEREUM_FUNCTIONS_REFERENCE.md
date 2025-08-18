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
"74010ece": "setMaxTxnAmount(uint256)",
"c860795d": "updateMaxTxn(uint256)",

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

## Contract Capability Analysis

### Understanding What Contracts Can Do

Not all contracts have the same capabilities. Understanding what a contract can actually do requires analyzing its code, storage patterns, and upgrade mechanisms.

#### Can All Contracts Create Hidden Mints?

**No, not all contracts can mint tokens.** Only contracts with specific characteristics can increase token supply:

1. **Explicit Mint Functions**
   ```python
   # Contracts with these functions can mint
   MINT_CAPABLE_FUNCTIONS = [
       "mint(address,uint256)",
       "_mint(address,uint256)", 
       "mintTokens(address,uint256)",
       "issue(address,uint256)",
       "create(address,uint256)"
   ]
   ```

2. **Upgradeable Contracts** (Most Dangerous)
   ```python
   # Proxy patterns that can add ANY function later
   UPGRADEABLE_PATTERNS = [
       "upgradeTo(address)",           # UUPS proxy
       "upgrade(address)",             # Transparent proxy
       "setImplementation(address)",   # Custom proxy
       "updateCode(address)"           # Diamond pattern
   ]
   ```

3. **Storage Manipulation**
   - Direct storage writes without function calls
   - Assembly/inline assembly usage
   - Delegatecall to external contracts

### Efficient Detection Methods

#### 1. Static Analysis - Extract All Functions
```python
from web3 import Web3
import re

def extract_all_functions(contract_address, w3):
    """Extract all function selectors from contract bytecode"""
    # Get contract bytecode
    bytecode = w3.eth.get_code(contract_address).hex()
    
    # Find all 4-byte sequences that look like function selectors
    # Look for PUSH4 instruction (63) followed by 4 bytes
    selectors = re.findall(r'63([0-9a-f]{8})', bytecode)
    
    # Also look for common patterns in Solidity bytecode
    # DUP1 PUSH4 pattern: 80 63
    selectors.extend(re.findall(r'8063([0-9a-f]{8})', bytecode))
    
    # Remove duplicates and format
    unique_selectors = list(set(selectors))
    return unique_selectors

def check_mint_capability(selectors):
    """Check if any selector matches known mint functions"""
    MINT_SELECTORS = {
        '40c10f19': 'mint(address,uint256)',
        '449a52f8': '_mint(address,uint256)',
        '0d61b519': 'mintTokens(address,uint256)',
        '867904b4': 'issue(address,uint256)',
        '1e4249fb': 'mintBatch(address[],uint256[])',
    }
    
    found_mints = []
    for selector in selectors:
        if selector in MINT_SELECTORS:
            found_mints.append(MINT_SELECTORS[selector])
    
    return found_mints
```

#### 2. Dynamic Analysis - Simulation
```python
def can_mint_tokens(contract_address, token_address, w3):
    """Test if contract can mint tokens through simulation"""
    # Get current total supply
    token_contract = w3.eth.contract(
        address=token_address,
        abi=[{"inputs": [], "name": "totalSupply", "outputs": [{"type": "uint256"}], "type": "function"}]
    )
    current_supply = token_contract.functions.totalSupply().call()
    
    # Common mint function signatures to test
    mint_calldata_samples = [
        # mint(address,uint256) with 1000 tokens to zero address
        '0x40c10f19' + '0' * 24 + '00000000000000000000000000000000' + 
        '00000000000000000000000000000000000000000000000003635c9adc5dea00000',
        
        # _mint(address,uint256)
        '0x4e6ec247' + '0' * 24 + '00000000000000000000000000000000' +
        '00000000000000000000000000000000000000000000000003635c9adc5dea00000',
    ]
    
    for calldata in mint_calldata_samples:
        try:
            # Simulate the mint call
            result = w3.eth.call({
                'to': contract_address,
                'data': calldata,
                'from': contract_address  # Call as the contract itself
            })
            
            # Check if supply would increase
            new_supply = token_contract.functions.totalSupply().call(
                block_identifier='latest'
            )
            
            if new_supply > current_supply:
                return True, "Can mint via standard functions"
                
        except Exception as e:
            continue
    
    return False, "No standard mint capability found"
```

#### 3. Proxy Detection
```python
def is_upgradeable_proxy(contract_address, w3):
    """Detect if contract is an upgradeable proxy"""
    # Check for proxy storage slots
    PROXY_SLOTS = {
        # EIP-1967 slots
        '0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc': 'IMPLEMENTATION_SLOT',
        '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103': 'ADMIN_SLOT',
        '0xa3f0ad74e5423aebfd80d3ef4346578335a9a72aeaee59ff6cb3582b35133d50': 'BEACON_SLOT',
    }
    
    for slot, slot_name in PROXY_SLOTS.items():
        value = w3.eth.get_storage_at(contract_address, slot)
        if int.from_bytes(value, 'big') != 0:
            implementation = '0x' + value.hex()[-40:]  # Last 20 bytes is address
            return True, f"Proxy detected via {slot_name}: {implementation}"
    
    # Check for delegatecall patterns in bytecode
    bytecode = w3.eth.get_code(contract_address).hex()
    if 'f4' in bytecode:  # DELEGATECALL opcode
        return True, "Contains DELEGATECALL - possible proxy"
    
    return False, "Not a proxy"
```

### Hidden Function Detection

#### 1. Obfuscated Functions
```python
def find_hidden_functions(contract_address, w3):
    """Find functions not exposed in standard interfaces"""
    bytecode = w3.eth.get_code(contract_address).hex()
    
    # Look for function patterns
    hidden_patterns = {
        # Functions called via assembly
        'f1': 'CALL opcode - external calls',
        'f2': 'CALLCODE opcode - deprecated dangerous',
        'f4': 'DELEGATECALL opcode - proxy calls',
        'fa': 'STATICCALL opcode - view calls',
        
        # Storage manipulation
        '55': 'SSTORE opcode - storage writes',
        '54': 'SLOAD opcode - storage reads',
        
        # Self destruct
        'ff': 'SELFDESTRUCT opcode - contract suicide',
    }
    
    findings = []
    for opcode, description in hidden_patterns.items():
        if opcode in bytecode:
            count = bytecode.count(opcode) // 2  # Hex pairs
            findings.append(f"{description}: {count} occurrences")
    
    return findings
```

#### 2. Storage-Based Capabilities
```python
def analyze_storage_patterns(contract_address, token_address, w3):
    """Detect if contract can manipulate token balances via storage"""
    # Standard balance storage slot calculation for mappings
    # mapping(address => uint256) typically at slot 0 or 1
    
    test_address = '0x000000000000000000000000000000000000dEaD'
    
    for slot in range(10):  # Check first 10 slots
        # Calculate storage key for mapping
        key = w3.keccak(
            bytes.fromhex(test_address[2:].zfill(64)) + 
            slot.to_bytes(32, 'big')
        )
        
        try:
            # Try to read balance
            value = w3.eth.get_storage_at(token_address, key)
            if int.from_bytes(value, 'big') > 0:
                return True, f"Balance mapping likely at slot {slot}"
        except:
            continue
    
    return False, "No standard balance mapping found"
```

### Contract Classification

#### Security Risk Levels
```python
def classify_contract_risk(contract_address, w3):
    """Classify contract by security risk level"""
    risk_score = 0
    risks = []
    
    # Check for mint capability
    selectors = extract_all_functions(contract_address, w3)
    mint_functions = check_mint_capability(selectors)
    if mint_functions:
        risk_score += 30
        risks.append(f"Mint functions: {mint_functions}")
    
    # Check if upgradeable
    is_proxy, proxy_info = is_upgradeable_proxy(contract_address, w3)
    if is_proxy:
        risk_score += 50
        risks.append(f"Upgradeable: {proxy_info}")
    
    # Check for dangerous opcodes
    hidden = find_hidden_functions(contract_address, w3)
    if any('SELFDESTRUCT' in h for h in hidden):
        risk_score += 40
        risks.append("Can self-destruct")
    
    if any('DELEGATECALL' in h for h in hidden):
        risk_score += 30
        risks.append("Uses delegatecall")
    
    # Classify
    if risk_score >= 70:
        return "CRITICAL", risks
    elif risk_score >= 40:
        return "HIGH", risks
    elif risk_score >= 20:
        return "MEDIUM", risks
    else:
        return "LOW", risks
```

### Real-Time Analysis

#### Mempool Simulation
```python
async def simulate_pending_transaction(tx_hash, w3):
    """Simulate a pending transaction to detect malicious behavior"""
    tx = w3.eth.get_transaction(tx_hash)
    
    # Get pre-state
    if 'mint' in tx.input.hex() or '40c10f19' in tx.input.hex():
        # This is a mint transaction - check authorization
        token_address = tx.to
        
        # Get current supply
        supply_before = get_total_supply(token_address, w3)
        
        # Simulate the transaction
        try:
            result = w3.eth.call({
                'from': tx['from'],
                'to': tx['to'],
                'data': tx['input'],
                'value': tx['value']
            }, 'pending')
            
            # Check post-state
            supply_after = get_total_supply(token_address, w3)
            
            if supply_after > supply_before:
                mint_amount = supply_after - supply_before
                mint_percent = (mint_amount / supply_before) * 100
                
                if mint_percent > 10:  # 10% supply increase
                    return {
                        'threat': 'MASSIVE_MINT',
                        'mint_percent': mint_percent,
                        'action': 'SELL_IMMEDIATELY'
                    }
        except Exception as e:
            return {'error': str(e)}
```

### Best Practices for Efficient Analysis

1. **Cache Results**: Store analysis results to avoid repeated computation
2. **Batch Calls**: Use multicall contracts for efficiency
3. **Focus on High-Risk Functions**: Prioritize checking critical functions
4. **Monitor Events**: Some capabilities are revealed through events
5. **Track Upgrades**: Monitor ProxyAdmin events for capability changes

## Usage Examples

### Complete Contract Analysis
```python
def comprehensive_contract_analysis(contract_address, w3):
    """Full security analysis of a contract"""
    print(f"Analyzing contract: {contract_address}")
    
    # 1. Extract all functions
    selectors = extract_all_functions(contract_address, w3)
    print(f"Found {len(selectors)} function selectors")
    
    # 2. Check mint capability
    mint_functions = check_mint_capability(selectors)
    if mint_functions:
        print(f"⚠️  MINT CAPABLE: {mint_functions}")
    
    # 3. Check if upgradeable
    is_proxy, proxy_info = is_upgradeable_proxy(contract_address, w3)
    if is_proxy:
        print(f"🚨 UPGRADEABLE: {proxy_info}")
    
    # 4. Find hidden functions
    hidden = find_hidden_functions(contract_address, w3)
    if hidden:
        print(f"🔍 Hidden capabilities: {hidden}")
    
    # 5. Overall risk assessment
    risk_level, risks = classify_contract_risk(contract_address, w3)
    print(f"\nRISK LEVEL: {risk_level}")
    for risk in risks:
        print(f"  - {risk}")
```

### Decoding Function Calls
```python
from web3 import Web3

def decode_function(input_data):
    """Extract function selector and decode common patterns"""
    if len(input_data) < 10:  # 0x + 8 chars
        return None
        
    selector = input_data[2:10]
    
    # Look up in function dictionary
    if selector == "02751cec":
        return {
            "function": "removeLiquidityETH",
            "critical": True,
            "params": decode_remove_liquidity_params(input_data)
        }
    # ... more decodings
```

### Generating Selectors
```python
# Generate 4-byte selector
w3 = Web3()
selector = w3.keccak(text="transfer(address,uint256)")[:4].hex()
print(f"transfer selector: 0x{selector}")  # 0xa9059cbb
```

This reference covers the complete spectrum of Ethereum functions across all major protocols and standards, serving as a comprehensive guide for blockchain analysis and development.

## Advanced Detection Techniques

### Tax Manipulation Detection
```python
def can_change_taxes(contract_address, w3):
    """Detect if contract can modify buy/sell taxes"""
    TAX_CHANGE_SELECTORS = {
        '8a8c523c': 'setFees(uint256,uint256)',
        '9012c4a8': 'updateFees(uint256,uint256)', 
        '7a806d6b': 'setTaxes(uint256,uint256)',
        '3b124fe7': 'setBuyTax(uint256)',
        '267822f3': 'setSellTax(uint256)',
    }
    
    selectors = extract_all_functions(contract_address, w3)
    found_tax_functions = []
    
    for selector in selectors:
        if selector in TAX_CHANGE_SELECTORS:
            found_tax_functions.append(TAX_CHANGE_SELECTORS[selector])
    
    # Also check for owner-only modifiers
    bytecode = w3.eth.get_code(contract_address).hex()
    has_owner = '8da5cb5b' in bytecode  # owner() function
    
    if found_tax_functions and has_owner:
        return True, f"Can change taxes via: {found_tax_functions}"
    
    return False, "No tax change capability found"
```

### Honeypot Detection
```python
def is_honeypot(token_address, w3):
    """Detect if token prevents selling (honeypot)"""
    # Method 1: Check for transfer restrictions
    indicators = []
    
    # Look for common honeypot patterns in bytecode
    bytecode = w3.eth.get_code(token_address).hex()
    
    honeypot_patterns = {
        # Common honeypot function selectors
        'e4849b32': 'sell(uint256)',  # Custom sell function
        'b515566a': 'setBots(address[])',  # Bot blacklist
        '6b9990cc': 'onlyOwnerCanTransfer',
        'd73dd623': 'restrictedTransfer',
    }
    
    for selector, name in honeypot_patterns.items():
        if selector in bytecode:
            indicators.append(f"Found {name} function")
    
    # Method 2: Simulate a sell transaction
    try:
        # Get a token holder address (check Transfer events)
        transfer_topic = w3.keccak(text="Transfer(address,address,uint256)").hex()
        logs = w3.eth.get_logs({
            'address': token_address,
            'topics': [transfer_topic],
            'fromBlock': 'latest',
            'toBlock': 'latest'
        })
        
        if logs:
            # Try to simulate a transfer from a holder
            holder = '0x' + logs[0]['topics'][2].hex()[26:]  # 'to' address
            
            # Simulate transfer back to contract
            transfer_data = '0xa9059cbb' + token_address[2:].zfill(64) + '0000000000000000000000000000000000000000000000000000000000000001'
            
            try:
                w3.eth.call({
                    'from': holder,
                    'to': token_address,
                    'data': transfer_data
                })
            except Exception as e:
                if 'revert' in str(e).lower():
                    indicators.append("Transfer simulation failed - possible honeypot")
    except:
        pass
    
    return len(indicators) > 0, indicators
```

### Liquidity Lock Detection
```python
def check_liquidity_lock_status(pair_address, w3):
    """Check if liquidity is locked and when it unlocks"""
    # Common liquidity locker contracts
    KNOWN_LOCKERS = {
        '0x663A5C229c09b049E36dCc11a9B0d4a8Eb9db214': 'Unicrypt',
        '0xE2fE530C047f2d85298b07D9333C05737f1435fB': 'Team Finance',
        '0x71B5759d73262FBb223956913ecF4ecC51057641': 'PinkLock',
    }
    
    # Get LP token balance for each locker
    lp_token = w3.eth.contract(
        address=pair_address,
        abi=[{"inputs": [{"type": "address"}], "name": "balanceOf", "outputs": [{"type": "uint256"}], "type": "function"}]
    )
    
    locked_info = []
    total_supply = lp_token.functions.totalSupply().call()
    
    for locker_address, locker_name in KNOWN_LOCKERS.items():
        try:
            balance = lp_token.functions.balanceOf(locker_address).call()
            if balance > 0:
                locked_percent = (balance / total_supply) * 100
                locked_info.append({
                    'locker': locker_name,
                    'locked_percent': locked_percent,
                    'locked_amount': balance
                })
        except:
            continue
    
    return locked_info
```

### Ownership Analysis
```python
def analyze_ownership_risks(contract_address, w3):
    """Analyze ownership-related risks"""
    risks = []
    
    # Check if ownership is renounced
    owner_slot = '0x0'  # Common owner storage slot
    owner = w3.eth.get_storage_at(contract_address, owner_slot)
    
    if owner == b'\x00' * 32:
        return ['Ownership renounced (good)']
    
    # Check what owner can do
    selectors = extract_all_functions(contract_address, w3)
    
    DANGEROUS_OWNER_FUNCTIONS = {
        '715018a6': 'setMaxTxAmount/renounceOwnership',
        'f2fde38b': 'transferOwnership',
        '8a8c523c': 'setFees/setTradingEnabled',
        '8456cb59': 'pause',
        '40c10f19': 'mint',
        'f9f92be4': 'blacklist',
    }
    
    for selector in selectors:
        if selector in DANGEROUS_OWNER_FUNCTIONS:
            risks.append(f"Owner can: {DANGEROUS_OWNER_FUNCTIONS[selector]}")
    
    return risks
```

### Gas-Efficient Batch Analysis
```python
def batch_analyze_tokens(token_addresses, w3):
    """Efficiently analyze multiple tokens in one call"""
    # Use multicall for efficiency
    multicall_abi = [{
        "inputs": [{"type": "tuple[]", "components": [{"type": "address"}, {"type": "bytes"}]}],
        "name": "aggregate",
        "outputs": [{"type": "uint256"}, {"type": "bytes[]"}],
        "type": "function"
    }]
    
    # Multicall3 address (same on all chains)
    multicall = w3.eth.contract(
        address='0xcA11bde05977b3631167028862bE2a173976CA11',
        abi=multicall_abi
    )
    
    # Build calls for each token
    calls = []
    for token in token_addresses:
        # Check total supply
        calls.append((token, '0x18160ddd'))  # totalSupply()
        # Check owner
        calls.append((token, '0x8da5cb5b'))  # owner()
    
    # Execute all calls in one transaction
    try:
        _, results = multicall.functions.aggregate(calls).call()
        
        # Process results
        analyzed = {}
        for i in range(0, len(results), 2):
            token = token_addresses[i // 2]
            total_supply = int.from_bytes(results[i], 'big')
            owner = '0x' + results[i + 1].hex()[-40:] if len(results[i + 1]) >= 20 else 'No owner'
            
            analyzed[token] = {
                'total_supply': total_supply,
                'owner': owner,
                'has_owner': owner != 'No owner' and owner != '0x' + '0' * 40
            }
            
        return analyzed
    except Exception as e:
        return {'error': str(e)}
```

### Event-Based Capability Detection
```python
def detect_capabilities_from_events(contract_address, w3, from_block='latest', to_block='latest'):
    """Detect contract capabilities from emitted events"""
    if from_block == 'latest':
        from_block = w3.eth.block_number - 1000  # Last 1000 blocks
    
    capabilities = set()
    
    # Get all events from contract
    logs = w3.eth.get_logs({
        'address': contract_address,
        'fromBlock': from_block,
        'toBlock': to_block
    })
    
    # Analyze event signatures
    for log in logs:
        if not log['topics']:
            continue
            
        event_sig = log['topics'][0].hex()
        
        # Map event signatures to capabilities
        if event_sig == w3.keccak(text="Transfer(address,address,uint256)").hex():
            capabilities.add('Can transfer tokens')
        elif event_sig == w3.keccak(text="Mint(address,uint256)").hex():
            capabilities.add('Has minted tokens')
        elif event_sig == w3.keccak(text="OwnershipTransferred(address,address)").hex():
            capabilities.add('Ownership is transferable')
        elif event_sig == w3.keccak(text="TaxUpdated(uint256,uint256)").hex():
            capabilities.add('Taxes have been changed')
        elif event_sig == w3.keccak(text="BlacklistUpdated(address,bool)").hex():
            capabilities.add('Has blacklist functionality')
    
    return list(capabilities)
```

## Performance Optimization Tips

### 1. Caching Strategy
```python
import time
from functools import lru_cache

class ContractAnalysisCache:
    def __init__(self, ttl=3600):  # 1 hour TTL
        self.cache = {}
        self.ttl = ttl
    
    def get(self, address, analysis_type):
        key = f"{address}:{analysis_type}"
        if key in self.cache:
            result, timestamp = self.cache[key]
            if time.time() - timestamp < self.ttl:
                return result
        return None
    
    def set(self, address, analysis_type, result):
        key = f"{address}:{analysis_type}"
        self.cache[key] = (result, time.time())

# Use with analysis functions
cache = ContractAnalysisCache()

def cached_analysis(contract_address, w3):
    # Check cache first
    cached = cache.get(contract_address, 'full_analysis')
    if cached:
        return cached
    
    # Perform analysis
    result = comprehensive_contract_analysis(contract_address, w3)
    
    # Cache result
    cache.set(contract_address, 'full_analysis', result)
    return result
```

### 2. Parallel Analysis
```python
import asyncio
from concurrent.futures import ThreadPoolExecutor

async def analyze_multiple_contracts(contracts, w3):
    """Analyze multiple contracts in parallel"""
    with ThreadPoolExecutor(max_workers=10) as executor:
        loop = asyncio.get_event_loop()
        
        tasks = [
            loop.run_in_executor(
                executor,
                comprehensive_contract_analysis,
                contract,
                w3
            )
            for contract in contracts
        ]
        
        results = await asyncio.gather(*tasks)
        return dict(zip(contracts, results))
```

## Key Takeaways

1. **Not all contracts can mint** - Only those with explicit mint functions or upgrade capabilities
2. **Proxies are the highest risk** - Can add any function at any time
3. **Static analysis has limits** - Some functions are hidden via assembly or delegatecall
4. **Simulation is powerful** - Test actual behavior rather than just reading code
5. **Monitor for upgrades** - Contract capabilities can change over time
6. **Events reveal behavior** - Historical events show what a contract has actually done
7. **Batch operations save gas** - Use multicall for analyzing multiple contracts
8. **Cache results** - Contract bytecode rarely changes, so cache analysis results