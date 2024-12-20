# Ethereum Transaction Receipt Data Structure

## Overview

A transaction receipt in Ethereum provides proof of transaction execution and its effects on the blockchain state. It contains crucial information about the transaction's execution, including status, gas usage, and emitted events.

## Core Components

### 1. Transaction Metadata
```json
{
    "transactionHash": "0x...",    // 32 bytes - Hash of the transaction
    "blockHash": "0x...",          // 32 bytes - Hash of the block containing this transaction
    "blockNumber": "0x...",        // Block number where this transaction was included
    "transactionIndex": "0x...",   // Position of the transaction in the block
    "from": "0x...",              // 20 bytes - Address of the sender
    "to": "0x..."                 // 20 bytes - Address of the recipient (null for contract creation)
}
```

### 2. Gas and Execution Data
```json
{
    "cumulativeGasUsed": "0x...",  // Total gas used in the block up to this transaction
    "gasUsed": "0x...",            // Gas used by this specific transaction
    "effectiveGasPrice": "0x...",  // Actual price paid per unit of gas
    "status": "0x1"                // 1 for success, 0 for failure
}
```

### 3. Contract Creation
```json
{
    "contractAddress": "0x..."     // Address of created contract (null if not contract creation)
}
```

### 4. Event Data
```json
{
    "logs": [...],                // Array of log entries generated during execution
    "logsBloom": "0x..."         // 256-byte bloom filter for log entry topics
}
```

## Transaction Logs

### Log Entry Structure
```json
{
    "address": "0x...",           // Contract that generated the event
    "topics": [                   // Array of 0-4 32-byte topics
        "0x...",                  // [0]: Event signature hash
        "0x...",                  // [1-3]: Indexed parameters (if any)
    ],
    "data": "0x...",             // ABI-encoded non-indexed parameters
    "blockNumber": "0x...",       // Block containing this log
    "transactionHash": "0x...",   // Transaction that generated this log
    "transactionIndex": "0x...",  // Transaction's index in the block
    "blockHash": "0x...",         // Hash of the containing block
    "logIndex": "0x...",         // Log's index in the transaction
    "removed": false              // True if log was removed due to chain reorg
}
```

## Common Event Signatures

### 1. ERC20 Token Events
```solidity
// Transfer event (keccak256 hash: 0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef)
event Transfer(address indexed from, address indexed to, uint256 value)

// Approval event (keccak256 hash: 0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925)
event Approval(address indexed owner, address indexed spender, uint256 value)
```

### 2. ERC721 Token Events
```solidity
// Transfer event (same signature hash as ERC20)
event Transfer(address indexed from, address indexed to, uint256 indexed tokenId)

// Approval event (different from ERC20 due to tokenId)
event Approval(address indexed owner, address indexed approved, uint256 indexed tokenId)
```

### 3. ERC1155 Token Events
```solidity
event TransferSingle(
    address indexed operator,
    address indexed from,
    address indexed to,
    uint256 id,
    uint256 value
)

event TransferBatch(
    address indexed operator,
    address indexed from,
    address indexed to,
    uint256[] ids,
    uint256[] values
)
```

## Important Considerations

### 1. Gas Calculations
- `gasUsed` represents actual gas consumed
- `effectiveGasPrice` * `gasUsed` = total transaction cost
- `cumulativeGasUsed` helps track block gas usage

### 2. Log Topics
- Maximum 4 topics per log (including event signature)
- Topics are always 32 bytes
- Only certain types can be indexed (address, numbers, bytes32)
- Dynamic types (string, arrays) are hashed when indexed

### 3. Chain Reorganizations
- `removed` flag indicates log validity
- Always check this flag when processing historical data
- Consider waiting for confirmations in real-time processing

### 4. Status Field
- Only available post-Byzantium fork
- Pre-Byzantium: success/failure determined by gasUsed
- Critical for transaction execution verification