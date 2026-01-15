# Secure Wallet Implementation Guide

## Overview

The ETH Kartal system now uses encrypted keystores instead of plain text private keys. This provides:

- **Encrypted storage** - Private keys are encrypted with AES-128-CTR
- **Password protection** - Keys require a password to decrypt
- **Memory safety** - Keys are cleared from memory when locked
- **Standard format** - Compatible with MetaMask, Geth, and other Ethereum tools

## Quick Start

### 1. Create a Keystore from Existing Private Key

```bash
# Build the keystore manager
cargo build --bin keystore_manager --release

# Create a keystore (you'll be prompted for private key and password)
./target/release/keystore_manager create -o ~/.eth_kartal/keystore.json

# Or provide private key via environment variable
ETH_PRIVATE_KEY="0x..." ./target/release/keystore_manager create -o ~/.eth_kartal/keystore.json
```

### 2. Verify Keystore

```bash
# Show address from keystore
./target/release/keystore_manager show -k ~/.eth_kartal/keystore.json

# Test keystore by signing a transaction
./target/release/keystore_manager test -k ~/.eth_kartal/keystore.json
```

### 3. Run ETH Kartal with Keystore

```bash
# Set keystore path
export ETH_KEYSTORE_PATH=~/.eth_kartal/keystore.json

# Run kartal (you'll be prompted for password)
cargo run --bin kartal --release

# Or specify keystore path directly
cargo run --bin kartal --release -- --keystore-path ~/.eth_kartal/keystore.json
```

## Integration Example

```rust
use eth_kartal::wallet::{SecureWallet, SecureWalletConfig, read_password};
use secrecy::Secret;

// Load wallet from keystore
let config = SecureWalletConfig {
    keystore_path: "/path/to/keystore.json".into(),
    chain_id: 1, // mainnet
};

let wallet = SecureWallet::from_keystore(config).await?;
println!("Loaded wallet for address: {}", wallet.address());

// Unlock with password
let password = read_password("Enter password: ")?;
wallet.unlock(password).await?;

// Now you can sign transactions
let signature = wallet.sign_transaction(&tx).await?;

// Lock when done (clears keys from memory)
wallet.lock().await;
```

## Running the Demo

```bash
# Run the secure wallet demo
cargo run --example secure_wallet_demo

# This will demonstrate:
# - Creating an encrypted keystore
# - Password validation
# - Transaction signing
# - Memory safety features
```

## Security Best Practices

1. **Never commit keystores** - Add `*.keystore` and `keystore.json` to `.gitignore`
2. **Use strong passwords** - At least 12 characters with mixed case, numbers, and symbols
3. **Limit keystore permissions** - `chmod 600 ~/.eth_kartal/keystore.json`
4. **Lock wallet when idle** - Call `wallet.lock()` when not actively signing
5. **Use hardware wallets in production** - This implementation is for development/testing

## Migration from Plain Text Keys

If you have existing plain text private keys:

1. Create a keystore using the keystore manager
2. Update your configuration to use `keystore_path` instead of `private_key`
3. Securely delete any files containing plain text private keys
4. Update any scripts or automation to use the new keystore format

## Keystore Format

The keystore uses the standard Ethereum keystore format (version 3):

```json
{
  "address": "...",
  "crypto": {
    "cipher": "aes-128-ctr",
    "cipherparams": { "iv": "..." },
    "ciphertext": "...",
    "kdf": "scrypt",
    "kdfparams": { "dklen": 32, "n": 8192, "p": 1, "r": 8, "salt": "..." },
    "mac": "..."
  },
  "id": "...",
  "version": 3
}
```

This format is compatible with:
- MetaMask (import via JSON file)
- Geth/OpenEthereum (place in keystore directory)
- Web3.py/Web3.js (load and decrypt)
- Hardware wallet software (for backup)

## Troubleshooting

### "Wallet is locked" error
- The wallet must be unlocked before signing transactions
- Call `executor.unlock_wallet(password)` after creating the executor

### "Invalid password" error
- Double-check your password (case-sensitive)
- Ensure you're using the correct keystore file
- Try the test command to verify the keystore

### Performance considerations
- Unlocking takes ~100-500ms due to key derivation
- Keep wallet unlocked during active trading sessions
- Lock wallet during idle periods for security