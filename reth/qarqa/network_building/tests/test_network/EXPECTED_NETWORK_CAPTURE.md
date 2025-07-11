# Expected Network Capture for Transaction 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae

## 📊 **Raw Transaction Data Analysis**

### **Transaction Overview**
- **Hash**: `0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae`
- **Type**: Multi-pool arbitrage/MEV transaction
- **Block**: 22646153 (Jun-06-2025 02:30:47 PM UTC)
- **From**: `0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1` (User)
- **To**: `0xfBd4cdB413E45a52E2C8312f670e9cE67E794C37` (Router)
- **Value**: 0.000000000022646153 ETH (negligible)
- **Gas Fee**: 0.003908755780679457 ETH

## 🔍 **Step-by-Step Network Construction**

### **Step 1: Parse All Internal ETH Transfers**
```
1. WETH → 0x6bDf3535...8f9d59e9d: 10.829495221098646603 ETH
2. 0x6bDf3535...8f9d59e9d → Uniswap V4: 10.829495221098646603 ETH
3. WETH → 0x3177F690...99FA1C359: 5.495899762937538401 ETH
4. 0x3177F690...99FA1C359 → Uniswap V4: 5.495899762937538401 ETH
```

### **Step 2: Parse All ERC-20 Token Transfers**
```
1. Uniswap V4 → 0x6bDf3535...8f9d59e9d: 27,158.423268 USDC
2. 0x6bDf3535...8f9d59e9d → 0xfBd4cdB4...67E794C37: 27,158.423268 USDC
3. 0xfBd4cdB4...67E794C37 → 0x6bDf3535...8f9d59e9d: 10.829495221098646603 WETH
4. Uniswap V3 USDT → 0xfBd4cdB4...67E794C37: 16,865.020704 USDT  
5. 0xfBd4cdB4...67E794C37 → Uniswap V3 USDT: 6.729614461788500138 WETH
6. Uniswap V4 → 0x3177F690...99FA1C359: 13,772.158296 USDT
7. 0x3177F690...99FA1C359 → 0xfBd4cdB4...67E794C37: 13,772.158296 USDT
8. 0xfBd4cdB4...67E794C37 → 0x3177F690...99FA1C359: 5.495899762937538401 WETH
```

### **Step 3: Calculate Net State Changes**

| Address | ETH In | ETH Out | Net ETH | WETH In | WETH Out | Net WETH | USDC In | USDC Out | Net USDC | USDT In | USDT Out | Net USDT |
|---------|--------|---------|---------|---------|----------|----------|---------|----------|----------|---------|----------|----------|
| **0x5B43...Ed1** (User) | 0 | 0.0039 | **-0.0039** | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **0x0000...000** (Network) | 0.0039 | 0 | **+0.0039** | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| **WETH Contract** | 0 | 16.325 | **-16.325** | 16.325 | 0 | **+16.325** | 0 | 0 | 0 | 0 | 0 | 0 |
| **Uniswap V4** | 16.325 | 0 | **+16.325** | 0 | 0 | 0 | 0 | 40,930 | **-40,930** | 0 | 13,772 | **-13,772** |
| **Uniswap V3 USDT** | 0 | 0 | 0 | 6.729 | 0 | **+6.729** | 0 | 0 | 0 | 16,865 | 0 | **+16,865** |
| **0x6bDf3535...** (Pass-through) | 10.829 | 10.829 | **0** | 10.829 | 10.829 | **0** | 27,158 | 27,158 | **0** | 0 | 0 | **0** |
| **0x3177F690...** (Pass-through) | 5.495 | 5.495 | **0** | 5.495 | 5.495 | **0** | 0 | 0 | **0** | 13,772 | 13,772 | **0** |
| **0xfBd4cdB4...** (Router) | 0 | 0 | **0** | 23.594 | 23.594 | **0** | 27,158 | 27,158 | **0** | 30,637 | 30,637 | **0** |

## ✅ **Expected Final Network (After Filtering)**

### **Meaningful Nodes (5 total)**
| Address | Entity Type | Role | Net Change |
|---------|-------------|------|------------|
| `0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1` | 👤 **User** | Transaction initiator | **-0.0039 ETH** (gas only) |
| `0x0000000000000000000000000000000000000000` | ⛏️ **Network** | Gas recipient | **+0.0039 ETH** (gas received) |
| `WETH_CONTRACT` | 🪙 **WETH** | ETH ↔ WETH conversion | **-16.325 ETH** → **+16.325 WETH** |
| `UNISWAP_V4_POOL` | 🏊 **Uniswap V4** | Pool Manager | **+16.325 ETH**, **-40,930 USDC**, **-13,772 USDT** |
| `UNISWAP_V3_USDT` | 🏊 **Uniswap V3** | USDT Pool | **+6.729 WETH**, **+16,865 USDT** |

### **Filtered Out (Zero Net Change)**
- ❌ `0x6bDf3535711ab1ac93b3e8de5A5682849f9d59e9d` - Pass-through (all flows cancel out)
- ❌ `0x3177F690b14f8a2731396b90C2c5c57A5C99FA1C359` - Pass-through (all flows cancel out)  
- ❌ `0xfBd4cdB413E45a52E2C8312f670e9cE67E794C37` - Router (complex routing but net zero)

### **Direct Fund Transfers (4 total)**

| From | To | ETH Amount | Stablecoin Amount | Other Tokens | Type |
|------|-----|------------|-------------------|--------------|------|
| 👤 **User**<br>`0x5B43...Ed1` | ⛏️ **Network**<br>`0x0000...000` | **0.0039 ETH** | - | - | 🔘 Gas Payment |
| 🪙 **WETH**<br>`WETH Contract` | 🏊 **Uniswap V4**<br>`Pool Manager` | **16.3254 ETH** | - | - | 🔵 ETH Transfer |
| 🏊 **Uniswap V4**<br>`Pool Manager` | 🏊 **Uniswap V3**<br>`USDT Pool` | - | **40,930 USDC**<br>**13,772 USDT** | - | 🟢 Stablecoin Transfer |
| 🏊 **Uniswap V3**<br>`USDT Pool` | 🪙 **WETH**<br>`WETH Contract` | **6.7296 ETH** | - | - | 🔵 ETH Transfer |

*Note: WETH amounts are shown as ETH (6.729 WETH → 6.7296 ETH)*

## 🔧 **Implementation Requirements**

### **1. Raw Data Parsing**
Our system must correctly parse:
```python
# Internal ETH transfers (4 total)
internal_transfers = [
    {"from": "WETH", "to": "0x6bDf3535...", "amount": "10.829495221098646603"},
    {"from": "0x6bDf3535...", "to": "Uniswap V4", "amount": "10.829495221098646603"},
    {"from": "WETH", "to": "0x3177F690...", "amount": "5.495899762937538401"},
    {"from": "0x3177F690...", "to": "Uniswap V4", "amount": "5.495899762937538401"}
]

# ERC-20 token transfers (8 total)
token_transfers = [
    {"from": "Uniswap V4", "to": "0x6bDf3535...", "token": "USDC", "amount": "27158.423268"},
    {"from": "0x6bDf3535...", "to": "0xfBd4cdB4...", "token": "USDC", "amount": "27158.423268"},
    {"from": "0xfBd4cdB4...", "to": "0x6bDf3535...", "token": "WETH", "amount": "10.829495221098646603"},
    {"from": "Uniswap V3", "to": "0xfBd4cdB4...", "token": "USDT", "amount": "16865.020704"},
    {"from": "0xfBd4cdB4...", "to": "Uniswap V3", "token": "WETH", "amount": "6.729614461788500138"},
    {"from": "Uniswap V4", "to": "0x3177F690...", "token": "USDT", "amount": "13772.158296"},
    {"from": "0x3177F690...", "to": "0xfBd4cdB4...", "token": "USDT", "amount": "13772.158296"},
    {"from": "0xfBd4cdB4...", "to": "0x3177F690...", "token": "WETH", "amount": "5.495899762937538401"}
]
```

### **2. State Change Calculation**
```python
def calculate_net_changes(transfers, token_transfers):
    # Aggregate all ins and outs per address
    # Calculate net changes for ETH, WETH, USDC, USDT
    # Apply threshold filtering (0.001 ETH equivalent)
    # Return only addresses with meaningful changes
```

### **3. WETH Treatment**
```python
def treat_weth_as_eth(node_data):
    # Convert WETH amounts to ETH
    # Combine with existing ETH amounts
    # Remove WETH from token displays
    # Show combined amount as ETH
```

### **4. Intermediary Filtering**
```python
def filter_intermediaries(nodes, threshold=0.001):
    meaningful_nodes = []
    for node in nodes:
        total_change = abs(node.net_eth) + abs(node.net_usdc/1000) + abs(node.net_usdt/1000)
        if total_change >= threshold:
            meaningful_nodes.append(node)
    return meaningful_nodes
```

## 🧪 **Validation Criteria**

### **✅ Test Assertions**
```python
def test_transaction_network():
    network = build_network("0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae")
    
    # Verify node count
    assert len(network.nodes) == 5
    
    # Verify edge count  
    assert len(network.edges) == 4
    
    # Verify intermediaries filtered
    node_addresses = {node.id for node in network.nodes}
    assert "0x6bDf3535711ab1ac93b3e8de5A5682849f9d59e9d" not in node_addresses
    assert "0x3177F690b14f8a2731396b90C2c5c57A5C99FA1C359" not in node_addresses
    assert "0xfBd4cdB413E45a52E2C8312f670e9cE67E794C37" not in node_addresses
    
    # Verify WETH treatment
    weth_edges = [e for e in network.edges if 'WETH' in str(e.tokenFlows)]
    assert len(weth_edges) == 0  # WETH should be converted to ETH
    
    # Verify total ETH volume
    total_eth = sum(float(e.ethAmount) for e in network.edges if e.movementTypes != 'Gas Payment')
    assert abs(total_eth - 23.0546) < 0.001  # 16.3254 + 6.7296 ≈ 23.055 ETH
```

## 🎯 **Expected Final Visualization**

The frontend should display:

### **Network Graph**
- **5 nodes** with proper icons and colors
- **4 edges** with appropriate thickness and colors
- **Clean layout** showing economic flows only

### **Direct Fund Transfers Table**
| From | To | ETH Amount | Stablecoin | Type |
|------|-----|------------|------------|------|
| 👤 User | ⛏️ Network | 0.0039 ETH | - | Gas |
| 🪙 WETH | 🏊 V4 Pool | 16.3254 ETH | - | ETH |
| 🏊 V4 Pool | 🏊 V3 Pool | - | 40,930 USDC + 13,772 USDT | Stables |
| 🏊 V3 Pool | 🪙 WETH | 6.7296 ETH | - | ETH |

### **Summary Stats**
- **Total Addresses**: 5 (meaningful only)
- **Fund Flow Edges**: 4 (excluding gas)
- **Total ETH Volume**: 23.055 ETH (16.3254 + 6.7296)
- **Transaction Status**: ✅ Success

This represents a clean, accurate view of the actual economic flows in this complex multi-pool arbitrage transaction, with all intermediary routing noise removed and WETH properly treated as ETH.