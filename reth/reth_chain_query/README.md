# Reth Chain Query
**High-performance blockchain data queries using direct Reth database access**

`reth_chain_query` provides lightning-fast blockchain data retrieval by querying Reth's MDBX database directly. Includes RethIndex, a complementary indexing layer that adds entity-centric queries missing from Reth's sequential storage.

## 📊 Architecture

```
┌─────────────────┐
│   Application   │
└────────┬────────┘
         │
┌────────▼────────┐
│   ChainQuery    │  ← Main interface
├─────────────────┤
│ - balance       │  ← ETH + Token balances
│ - storage       │  ← Direct slot access
│ - block         │  ← Block metadata
│ - transaction   │  ← Transaction queries
│ - reth_index    │  ← Analytics indexes
└────────┬────────┘
         │
┌────────▼────────┐
│  TxSimulator    │  ← Database gateway
└────────┬────────┘
         │
┌────────▼────────┐
│  Reth Provider  │  ← State abstraction
└────────┬────────┘
         │
┌────────▼────────┐
│   MDBX Database │  ← Reth's blockchain data
│   (B+ Tree)     │  ← Memory-mapped I/O
└─────────────────┘
```

## 📚 Complete Reth Database Tables Reference

Understanding Reth's database structure is crucial for optimizing queries. Reth uses MDBX with B+ trees for logarithmic lookups and memory-mapped I/O for zero-copy access.

### Block and Header Tables (7 tables)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **Headers** | `BlockNumber` → `Header` | Complete block headers with metadata |
| **CanonicalHeaders** | `BlockNumber` → `HeaderHash` | Canonical chain mapping |
| **HeaderNumbers** | `BlockHash` → `BlockNumber` | Reverse hash lookup |
| **HeaderTerminalDifficulties** | `BlockNumber` → `CompactU256` | PoW→PoS transition data |
| **BlockBodyIndices** | `BlockNumber` → `{first_tx_num, tx_count}` | Transaction ranges per block |
| **BlockOmmers** | `BlockNumber` → `Vec<Header>` | Uncle blocks (pre-merge) |
| **BlockWithdrawals** | `BlockNumber` → `Vec<Withdrawal>` | Validator withdrawals (post-merge) |

### Transaction Tables (5 tables)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **Transactions** | `TxNumber` → `TransactionSigned` | Full transaction data |
| **TransactionHashNumbers** | `TxHash` → `TxNumber` | Hash to sequential ID mapping |
| **TransactionBlocks** | `TxNumber` → `BlockNumber` | Transaction to block mapping* |
| **TransactionSenders** | `TxNumber` → `Address` | Cached sender addresses |
| **Receipts** | `TxNumber` → `Receipt` | Logs, gas used, status |

*Note: Key is the highest TxNumber in the block

### Current State Tables (4 tables)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **PlainAccountState** | `Address` → `{nonce, balance, code_hash}` | Current ETH balances |
| **PlainStorageState** | `(Address, StorageKey)` → `Value` | Current storage values** |
| **Bytecodes** | `CodeHash` → `Bytecode` | Contract code |
| **HashedAccounts** | `Keccak(Address)` → `Account` | For state root calculation |

**Includes all token balances, DEX states, contract storage

### Historical State Tables (4 tables - Archive Node Only)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **AccountsHistory** | `ShardedKey<Address>` → `BlockNumberList` | When ETH balance changed |
| **StoragesHistory** | `StorageShardedKey` → `BlockNumberList` | When storage changed |
| **AccountChangeSets** | `(BlockNumber, Address)` → `Account` | ETH balance before change |
| **StorageChangeSets** | `(BlockNumber, Address, Key)` → `Value` | Storage before change |

### Merkle Trie Tables (4 tables)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **HashedStorages** | `(Keccak(Address), Keccak(Key))` → `Value` | Hashed storage for tries |
| **AccountsTrie** | `StoredNibbles` → `BranchNode` | Account trie nodes |
| **StoragesTrie** | `(B256, StoredNibblesSubKey)` → `Node` | Storage trie nodes |

### System Tables (3 tables)

| Table | Key → Value | Purpose |
|-------|-------------|---------|
| **StageCheckpoints** | `StageId` → `StageCheckpoint` | Sync progress tracking |
| **StageCheckpointProgresses** | `StageId` → `Vec<u8>` | Detailed stage progress |
| **PruneCheckpoints** | `PruneSegment` → `PruneCheckpoint` | Pruning progress |

## 🏗️ Module Structure
