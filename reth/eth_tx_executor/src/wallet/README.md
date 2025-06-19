# Wallet Module

## Overview
The Wallet module handles secure private key management, transaction signing, and account balance tracking. It provides a safe interface for interacting with Ethereum accounts while protecting sensitive key material.

## Components

### `manager.rs`
- Manages wallet initialization and key loading
- Provides transaction signing interface
- Tracks account balances and nonces
- Handles multiple wallet support

## Security Architecture

### Key Storage Options
1. **Environment Variable** (Development)
   ```bash
   export ETH_KARTAL_PRIVATE_KEY="0x..."
   ```

2. **Encrypted File** (Staging)
   ```rust
   pub struct EncryptedKeystore {
       path: PathBuf,
       password: SecureString,
   }
   ```

3. **Hardware Wallet** (Production)
   ```rust
   pub struct HardwareWallet {
       device_type: WalletType::Ledger,
       derivation_path: "m/44'/60'/0'/0/0",
   }
   ```

### Key Protection
- Never log private keys
- Clear key material from memory after use
- Use secure random for nonce generation
- Validate all signing requests

## Wallet Interface

### Basic Operations
```rust
pub trait Wallet {
    /// Get the wallet address
    fn address(&self) -> Address;
    
    /// Sign a transaction
    async fn sign_transaction(&self, tx: TransactionRequest) -> Result<SignedTransaction>;
    
    /// Get current nonce
    async fn get_nonce(&self) -> Result<U256>;
    
    /// Get ETH balance
    async fn get_balance(&self) -> Result<U256>;
    
    /// Get token balance
    async fn get_token_balance(&self, token: Address) -> Result<U256>;
}
```

### Transaction Signing
```rust
pub struct SigningRequest {
    pub transaction: TransactionRequest,
    pub chain_id: u64,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee: Option<U256>,
}

pub struct SignedTransaction {
    pub raw_transaction: Bytes,
    pub hash: H256,
    pub from: Address,
    pub nonce: U256,
}
```

## Nonce Management

### Strategy
- Track nonces locally for speed
- Sync with chain periodically
- Handle nonce gaps from failed transactions
- Support concurrent transaction signing

### Implementation
```rust
pub struct NonceManager {
    current_nonce: AtomicU64,
    pending_transactions: HashMap<U256, TxHash>,
    last_sync: Instant,
}
```

## Multi-Wallet Support

### Use Cases
- **Hot Wallet**: For small, frequent transactions
- **Cold Wallet**: For large, infrequent transactions
- **Dedicated Wallets**: Per-strategy isolation

### Configuration
```toml
[[wallets]]
name = "hot"
type = "local"
max_value_eth = 1.0

[[wallets]]
name = "main"
type = "hardware"
max_value_eth = 100.0
requires_confirmation = true
```

## Balance Tracking

### Monitored Assets
- ETH balance
- ERC-20 token balances
- Pending transaction values
- Available vs. total balance

### Update Triggers
- Before transaction submission
- After transaction confirmation
- On strategy evaluation
- Periodic refresh (configurable)

## Usage Example
```rust
use eth_kartal::wallet::{WalletManager, SigningRequest};

// Initialize wallet manager
let wallet_manager = WalletManager::from_env()?;

// Get wallet info
let address = wallet_manager.address();
let balance = wallet_manager.get_balance().await?;
println!("Wallet {} has {} ETH", address, balance);

// Sign a transaction
let tx_request = TransactionRequest::new()
    .to(router_address)
    .value(0)
    .data(swap_calldata);

let signed_tx = wallet_manager.sign_transaction(tx_request).await?;

// Submit to network
let tx_hash = provider.send_raw_transaction(signed_tx.raw_transaction).await?;
```

## Security Best Practices

### Do's
- ✅ Use hardware wallets in production
- ✅ Validate all addresses before signing
- ✅ Implement transaction value limits
- ✅ Log transaction hashes, not keys
- ✅ Use secure communication channels
- ✅ Implement key rotation procedures

### Don'ts
- ❌ Store private keys in code
- ❌ Log sensitive key material
- ❌ Reuse nonces
- ❌ Skip transaction simulation
- ❌ Ignore balance checks
- ❌ Use unencrypted key storage

## Error Handling
- **Invalid Key**: Fail fast with clear error
- **Insufficient Balance**: Return specific error type
- **Nonce Too Low**: Fetch fresh nonce and retry
- **Signing Failed**: Log error without key details

## Integration Points
- **Input**: Transaction requests from executor
- **Output**: Signed transactions ready for submission
- **Dependencies**: Web3 provider for balance/nonce queries

## Performance Considerations
- Nonce caching reduces RPC calls
- Balance updates batched when possible
- Signing is CPU-bound (~10ms)
- Hardware wallet signing slower (~1s)

## Monitoring
- Track signing request rate
- Monitor balance changes
- Alert on low balance
- Log all transaction signatures
- Track nonce gaps/issues