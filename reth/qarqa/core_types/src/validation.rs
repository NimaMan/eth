//! Input validation and security measures for QARQA

use crate::{QarqaError, QarqaResult};
use alloy_primitives::{Address, B256, U256};
use std::str::FromStr;
use tracing::warn;

/// Validation rules for different input types
pub struct ValidationRules {
    /// Maximum allowed transaction depth for analysis
    pub max_depth: u32,
    /// Maximum addresses to analyze in batch
    pub max_batch_size: usize,
    /// Maximum time range in blocks
    pub max_block_range: u64,
    /// Maximum value for filtering (in ETH)
    pub max_eth_value: f64,
    /// Minimum value for filtering (in ETH)
    pub min_eth_value: f64,
    /// Maximum string length for labels
    pub max_label_length: usize,
    /// Allowed characters in labels (regex pattern)
    pub label_pattern: regex::Regex,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_batch_size: 1000,
            max_block_range: 100_000, // ~2 weeks
            max_eth_value: 1_000_000.0,
            min_eth_value: 0.0,
            max_label_length: 256,
            label_pattern: regex::Regex::new(r"^[a-zA-Z0-9_\-\.\s]+$").unwrap(),
        }
    }
}

/// Input validator with configurable rules
pub struct InputValidator {
    rules: ValidationRules,
}

impl InputValidator {
    pub fn new() -> Self {
        Self {
            rules: ValidationRules::default(),
        }
    }
    
    pub fn with_rules(rules: ValidationRules) -> Self {
        Self { rules }
    }
    
    /// Validate Ethereum address
    pub fn validate_address(&self, address_str: &str) -> QarqaResult<Address> {
        // Remove common prefixes/suffixes
        let cleaned = address_str.trim();
        
        // Check basic format
        if !cleaned.starts_with("0x") && !cleaned.starts_with("0X") {
            return Err(QarqaError::InvalidInput(
                format!("Address must start with '0x', got: '{}'", cleaned)
            ));
        }
        
        // Check length (0x + 40 hex chars)
        if cleaned.len() != 42 {
            return Err(QarqaError::InvalidInput(
                format!("Address must be 42 characters (0x + 40 hex), got {} characters", cleaned.len())
            ));
        }
        
        // Check hex characters
        let hex_part = &cleaned[2..];
        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(QarqaError::InvalidInput(
                "Address contains invalid hex characters".to_string()
            ));
        }
        
        // Parse address
        Address::from_str(cleaned)
            .map_err(|e| QarqaError::InvalidInput(format!("Invalid address: {}", e)))
    }
    
    /// Validate transaction hash
    pub fn validate_transaction_hash(&self, hash_str: &str) -> QarqaResult<B256> {
        let cleaned = hash_str.trim();
        
        // Check format
        if !cleaned.starts_with("0x") && !cleaned.starts_with("0X") {
            return Err(QarqaError::InvalidInput(
                format!("Transaction hash must start with '0x', got: '{}'", cleaned)
            ));
        }
        
        // Check length (0x + 64 hex chars)
        if cleaned.len() != 66 {
            return Err(QarqaError::InvalidInput(
                format!("Transaction hash must be 66 characters (0x + 64 hex), got {} characters", cleaned.len())
            ));
        }
        
        // Parse hash
        B256::from_str(cleaned)
            .map_err(|e| QarqaError::InvalidInput(format!("Invalid transaction hash: {}", e)))
    }
    
    /// Validate analysis depth
    pub fn validate_depth(&self, depth: u32) -> QarqaResult<u32> {
        if depth == 0 {
            return Err(QarqaError::InvalidInput(
                "Analysis depth must be at least 1".to_string()
            ));
        }
        
        if depth > self.rules.max_depth {
            return Err(QarqaError::InvalidInput(
                format!("Analysis depth {} exceeds maximum allowed depth of {}", 
                        depth, self.rules.max_depth)
            ));
        }
        
        Ok(depth)
    }
    
    /// Validate batch size
    pub fn validate_batch_size(&self, size: usize) -> QarqaResult<usize> {
        if size == 0 {
            return Err(QarqaError::InvalidInput(
                "Batch size must be at least 1".to_string()
            ));
        }
        
        if size > self.rules.max_batch_size {
            return Err(QarqaError::InvalidInput(
                format!("Batch size {} exceeds maximum allowed size of {}", 
                        size, self.rules.max_batch_size)
            ));
        }
        
        Ok(size)
    }
    
    /// Validate block range
    pub fn validate_block_range(&self, start: u64, end: u64) -> QarqaResult<(u64, u64)> {
        if start > end {
            return Err(QarqaError::InvalidInput(
                format!("Start block {} must be less than or equal to end block {}", start, end)
            ));
        }
        
        let range = end - start;
        if range > self.rules.max_block_range {
            return Err(QarqaError::InvalidInput(
                format!("Block range {} exceeds maximum allowed range of {}", 
                        range, self.rules.max_block_range)
            ));
        }
        
        Ok((start, end))
    }
    
    /// Validate ETH value
    pub fn validate_eth_value(&self, value: f64) -> QarqaResult<f64> {
        if value < self.rules.min_eth_value {
            return Err(QarqaError::InvalidInput(
                format!("ETH value {} is below minimum of {}", 
                        value, self.rules.min_eth_value)
            ));
        }
        
        if value > self.rules.max_eth_value {
            return Err(QarqaError::InvalidInput(
                format!("ETH value {} exceeds maximum of {}", 
                        value, self.rules.max_eth_value)
            ));
        }
        
        if !value.is_finite() {
            return Err(QarqaError::InvalidInput(
                "ETH value must be a finite number".to_string()
            ));
        }
        
        Ok(value)
    }
    
    /// Validate and sanitize label
    pub fn validate_label(&self, label: &str) -> QarqaResult<String> {
        let trimmed = label.trim();
        
        if trimmed.is_empty() {
            return Ok(String::new());
        }
        
        if trimmed.len() > self.rules.max_label_length {
            return Err(QarqaError::InvalidInput(
                format!("Label length {} exceeds maximum of {}", 
                        trimmed.len(), self.rules.max_label_length)
            ));
        }
        
        if !self.rules.label_pattern.is_match(trimmed) {
            return Err(QarqaError::InvalidInput(
                format!("Label contains invalid characters. Only alphanumeric, underscore, hyphen, dot, and space allowed")
            ));
        }
        
        Ok(trimmed.to_string())
    }
    
    /// Validate U256 value from string
    pub fn validate_u256(&self, value_str: &str) -> QarqaResult<U256> {
        let cleaned = value_str.trim();
        
        // Check for negative values
        if cleaned.starts_with('-') {
            return Err(QarqaError::InvalidInput(
                "U256 value cannot be negative".to_string()
            ));
        }
        
        // Try to parse
        U256::from_str(cleaned)
            .map_err(|e| QarqaError::InvalidInput(format!("Invalid U256 value: {}", e)))
    }
    
    /// Sanitize SQL input to prevent injection
    pub fn sanitize_sql_identifier(&self, identifier: &str) -> QarqaResult<String> {
        let cleaned = identifier.trim();
        
        // Only allow alphanumeric and underscore
        if !cleaned.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(QarqaError::InvalidInput(
                "SQL identifier contains invalid characters".to_string()
            ));
        }
        
        // Check reserved words
        let reserved = ["SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER"];
        if reserved.iter().any(|&word| cleaned.eq_ignore_ascii_case(word)) {
            return Err(QarqaError::InvalidInput(
                format!("'{}' is a reserved SQL word", cleaned)
            ));
        }
        
        Ok(cleaned.to_lowercase())
    }
}

/// Security utilities
pub mod security {
    use super::*;
    use sha2::{Sha256, Digest};
    
    /// Hash sensitive data for logging
    pub fn hash_for_logging(data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        format!("SHA256:{}", hex::encode(&result[..8])) // First 8 bytes
    }
    
    /// Redact sensitive parts of address for logging
    pub fn redact_address(address: &Address) -> String {
        let addr_str = format!("{:?}", address);
        format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len()-4..])
    }
    
    /// Check if request seems malicious
    pub fn detect_suspicious_pattern(input: &str) -> bool {
        let suspicious_patterns = [
            "';", "--;", "/*", "*/", "xp_", "sp_",
            "UNION", "EXEC", "EXECUTE", "SCRIPT",
            "<script", "javascript:", "onerror=",
            "../", "..\\", "%2e%2e", "%252e%252e"
        ];
        
        let lower = input.to_lowercase();
        for pattern in &suspicious_patterns {
            if lower.contains(&pattern.to_lowercase()) {
                warn!("Suspicious pattern detected: {}", pattern);
                return true;
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_validation() {
        let validator = InputValidator::new();
        
        // Valid address
        let valid = validator.validate_address("0x742d35Cc6634C0532925a3b844Bc9e7595f5b899");
        assert!(valid.is_ok());
        
        // Invalid addresses
        assert!(validator.validate_address("").is_err());
        assert!(validator.validate_address("0x").is_err());
        assert!(validator.validate_address("742d35Cc6634C0532925a3b844Bc9e7595f5b899").is_err());
        assert!(validator.validate_address("0xGGGG35Cc6634C0532925a3b844Bc9e7595f5b899").is_err());
        assert!(validator.validate_address("0x742d35Cc6634C0532925a3b844Bc9e7595f5b89").is_err());
    }
    
    #[test]
    fn test_transaction_hash_validation() {
        let validator = InputValidator::new();
        
        // Valid hash
        let valid = validator.validate_transaction_hash(
            "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006"
        );
        assert!(valid.is_ok());
        
        // Invalid hashes
        assert!(validator.validate_transaction_hash("").is_err());
        assert!(validator.validate_transaction_hash("0x").is_err());
        assert!(validator.validate_transaction_hash("not-a-hash").is_err());
    }
    
    #[test]
    fn test_sql_sanitization() {
        let validator = InputValidator::new();
        
        // Valid identifiers
        assert_eq!(validator.sanitize_sql_identifier("users").unwrap(), "users");
        assert_eq!(validator.sanitize_sql_identifier("user_id").unwrap(), "user_id");
        
        // Invalid identifiers
        assert!(validator.sanitize_sql_identifier("DROP TABLE users").is_err());
        assert!(validator.sanitize_sql_identifier("users; DROP TABLE").is_err());
        assert!(validator.sanitize_sql_identifier("SELECT").is_err());
    }
    
    #[test]
    fn test_suspicious_pattern_detection() {
        assert!(security::detect_suspicious_pattern("'; DROP TABLE users--"));
        assert!(security::detect_suspicious_pattern("<script>alert('xss')</script>"));
        assert!(security::detect_suspicious_pattern("../../etc/passwd"));
        assert!(!security::detect_suspicious_pattern("normal input"));
    }
}