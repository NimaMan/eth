/// Generic storage query utilities
/// 
/// Provides low-level storage access for arbitrary contract storage slots

use alloy_primitives::{Address, U256, B256};
use eyre::Result;
use tx_simulator::TxSimulator;
use std::sync::Arc;
use tiny_keccak::{Hasher, Keccak};

/// Storage query module
pub struct StorageQuery {
    simulator: Arc<TxSimulator>,
}

impl StorageQuery {
    /// Create new StorageQuery instance
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
        Self { simulator }
    }
    
    /// Read storage at a specific slot
    pub async fn get_storage_at(&self, address: Address, slot: B256, block_number: Option<u64>) -> Result<U256> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.get_chain_state_at_block(block_number)
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Read storage
        let value = provider.storage(address, slot)
            .map_err(|e| eyre::eyre!("Failed to read storage: {}", e))?;
        
        Ok(value.unwrap_or(U256::ZERO))
    }
    
    /// Read storage at a numeric slot index
    pub async fn get_storage_at_index(&self, address: Address, index: u64, block_number: Option<u64>) -> Result<U256> {
        let slot = B256::from(U256::from(index));
        self.get_storage_at(address, slot, block_number).await
    }
    
    /// Calculate storage slot for a single-key mapping
    /// Used for mappings like: mapping(address => uint256)
    pub fn calculate_mapping_slot(key: Address, mapping_slot: u64) -> B256 {
        let mut encoded = [0u8; 64];
        // First 32 bytes: the key (address padded to 32 bytes)
        encoded[12..32].copy_from_slice(key.as_slice());
        // Second 32 bytes: the mapping slot
        let slot_bytes = U256::from(mapping_slot).to_be_bytes::<32>();
        encoded[32..64].copy_from_slice(&slot_bytes);
        
        let mut hasher = Keccak::v256();
        hasher.update(&encoded);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        
        B256::from(hash)
    }
    
    /// Calculate storage slot for a double-key mapping
    /// Used for mappings like: mapping(address => mapping(address => uint256))
    pub fn calculate_double_mapping_slot(key1: Address, key2: Address, mapping_slot: u64) -> B256 {
        // First hash: keccak256(key1 . mapping_slot)
        let mut first_encoded = [0u8; 64];
        first_encoded[12..32].copy_from_slice(key1.as_slice());
        let slot_bytes = U256::from(mapping_slot).to_be_bytes::<32>();
        first_encoded[32..64].copy_from_slice(&slot_bytes);
        
        let mut hasher = Keccak::v256();
        hasher.update(&first_encoded);
        let mut first_hash = [0u8; 32];
        hasher.finalize(&mut first_hash);
        
        // Second hash: keccak256(key2 . first_hash)
        let mut second_encoded = [0u8; 64];
        second_encoded[12..32].copy_from_slice(key2.as_slice());
        second_encoded[32..64].copy_from_slice(&first_hash);
        
        let mut hasher = Keccak::v256();
        hasher.update(&second_encoded);
        let mut final_hash = [0u8; 32];
        hasher.finalize(&mut final_hash);
        
        B256::from(final_hash)
    }
    
    /// Calculate storage slot for array element
    /// Used for dynamic arrays: array[index]
    pub fn calculate_array_slot(array_slot: u64, index: u64) -> B256 {
        // The array length is stored at array_slot
        // Elements are stored at keccak256(array_slot) + index
        
        let slot_bytes = U256::from(array_slot).to_be_bytes::<32>();
        
        let mut hasher = Keccak::v256();
        hasher.update(&slot_bytes);
        let mut base_hash = [0u8; 32];
        hasher.finalize(&mut base_hash);
        
        // Add index to base hash
        let base = U256::from_be_bytes(base_hash);
        let final_slot = base + U256::from(index);
        
        B256::from(final_slot.to_be_bytes::<32>())
    }
    
    /// Read multiple storage slots in a single batch
    pub async fn get_storage_batch(&self, address: Address, slots: Vec<B256>, block_number: Option<u64>) -> Result<Vec<U256>> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.get_chain_state_at_block(block_number)
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Read all slots
        let mut results = Vec::with_capacity(slots.len());
        for slot in slots {
            let value = provider.storage(address, slot)
                .map_err(|e| eyre::eyre!("Failed to read storage at slot {:?}: {}", slot, e))?;
            results.push(value.unwrap_or(U256::ZERO));
        }
        
        Ok(results)
    }
}