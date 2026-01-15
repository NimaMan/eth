//! Bundle signing for Flashbots authentication

use ethers::prelude::*;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::utils::keccak256;
use serde_json;

use super::types::BundleRequest;

/// Bundle signer for Flashbots reputation system
#[derive(Clone, Debug)]
pub struct BundleSigner {
    /// Private key for signing
    wallet: LocalWallet,
}

impl BundleSigner {
    /// Create new signer from private key
    pub fn new(private_key: SigningKey) -> Self {
        Self {
            wallet: LocalWallet::from(private_key),
        }
    }
    
    /// Create random signer (for testing only)
    pub fn random() -> Self {
        Self {
            wallet: LocalWallet::new(&mut rand::thread_rng()),
        }
    }
    
    /// Load signer from keystore
    pub async fn from_keystore(
        path: &str,
        password: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let wallet = LocalWallet::decrypt_keystore(path, password)?;
        Ok(Self { wallet })
    }
    
    /// Get signer address
    pub fn address(&self) -> Address {
        self.wallet.address()
    }
    
    /// Sign bundle for submission
    pub fn sign_bundle(
        &self,
        bundle: &BundleRequest,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Serialize bundle to JSON for signing
        let bundle_json = serde_json::to_string(bundle)?;
        let bundle_hash = keccak256(bundle_json.as_bytes());
        
        // Sign the hash
        let signature = self.sign_message(&bundle_hash)?;
        
        Ok(signature)
    }
    
    /// Sign arbitrary message
    pub fn sign_message(
        &self,
        message: &[u8],
    ) -> Result<String, Box<dyn std::error::Error>> {
        // EIP-191 personal message signing
        let message_hash = hash_message(message);
        
        // Sign with wallet
        let signature = self.wallet.sign_hash(message_hash)?;
        
        // Convert to hex string
        Ok(format!("0x{}", hex::encode(signature.to_vec())))
    }
    
    /// Sign bundle hash directly
    pub fn sign_bundle_hash(
        &self,
        bundle_hash: H256,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.sign_message(bundle_hash.as_bytes())
    }
}

/// Hash message according to EIP-191
fn hash_message(message: &[u8]) -> H256 {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut bytes = prefix.as_bytes().to_vec();
    bytes.extend_from_slice(message);
    keccak256(&bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bundle_signer() {
        let signer = BundleSigner::random();
        let message = b"test message";
        
        let signature = signer.sign_message(message).unwrap();
        assert!(signature.starts_with("0x"));
        assert_eq!(signature.len(), 132); // 0x + 65 bytes * 2
    }
    
    #[test]
    fn test_sign_bundle() {
        let signer = BundleSigner::random();
        
        let bundle = BundleRequest {
            txs: vec!["0x1234".to_string()],
            block_number: U256::from(12345),
            min_timestamp: None,
            max_timestamp: None,
            reverting_tx_hashes: None,
        };
        
        let signature = signer.sign_bundle(&bundle).unwrap();
        assert!(signature.starts_with("0x"));
    }
}