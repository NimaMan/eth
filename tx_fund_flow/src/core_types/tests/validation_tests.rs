//! Comprehensive validation tests for TX_FUND_FLOW core types
//! 
//! Tests input validation, security measures, and edge cases with real-world data patterns

use tx_fund_flow_core_types::*;
use tx_fund_flow_core_types::validation::*;
use alloy_primitives::{Address, B256, U256};
use std::str::FromStr;

#[cfg(test)]
mod address_validation_tests {
    use super::*;

    #[test]
    fn test_valid_ethereum_addresses() {
        let validator = InputValidator::new();
        
        // Test cases with real mainnet addresses
        let valid_addresses = vec![
            // Standard addresses
            ("0x0000000000000000000000000000000000000000", "Zero address"),
            ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT contract"),
            ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC contract"),
            ("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", "WETH contract"),
            
            // Checksum addresses (EIP-55)
            ("0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed", "Checksum example 1"),
            ("0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359", "Checksum example 2"),
            ("0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB", "Checksum example 3"),
            ("0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb", "Checksum example 4"),
            
            // Known important addresses
            ("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", "vitalik.eth"),
            ("0xBE0eB53F46cd790Cd13851d5EFf43D12404d33E8", "Binance 7"),
            ("0xC61b9BB3A7a0767E3179713f3A5c7a9aeDCE193C", "Bitfinex 3"),
        ];
        
        for (address, description) in valid_addresses {
            let result = validator.validate_address(address);
            assert!(result.is_ok(), "Failed to validate {}: {}", description, result.err().unwrap());
            
            let parsed = result.unwrap();
            // Addresses should be normalized to lowercase internally
            assert_eq!(
                format!("{:?}", parsed).to_lowercase(),
                address.to_lowercase(),
                "Address normalization failed for {}", description
            );
        }
    }

    #[test]
    fn test_invalid_address_formats() {
        let validator = InputValidator::new();
        
        let invalid_addresses = vec![
            ("", "Empty string"),
            ("0x", "Only prefix"),
            ("0", "Too short"),
            ("x0000000000000000000000000000000000000000", "Wrong prefix"),
            ("00000000000000000000000000000000000000000", "Missing prefix"),
            ("0x00000000000000000000000000000000000000", "Too short (38 chars)"),
            ("0x000000000000000000000000000000000000000000", "Too long (44 chars)"),
            ("0xGGGG000000000000000000000000000000000000", "Invalid hex chars"),
            ("0x00000000000000000000000000000000000000zz", "Invalid hex at end"),
            ("0x 0000000000000000000000000000000000000000", "Space in address"),
            ("0x00000000000000000000000000000000000000\n00", "Newline in address"),
        ];
        
        for (address, description) in invalid_addresses {
            let result = validator.validate_address(address);
            assert!(result.is_err(), "Should reject {}: {}", description, address);
            
            if let Err(QarqaError::InvalidInput(msg)) = result {
                assert!(msg.len() > 0, "Error message should not be empty for {}", description);
            } else {
                panic!("Wrong error type for {}", description);
            }
        }
    }

    #[test]
    fn test_address_sql_injection_prevention() {
        let validator = InputValidator::new();
        
        let sql_injection_attempts = vec![
            "0x'; DROP TABLE users; --",
            "0x' OR '1'='1",
            "0x\"; DELETE FROM accounts WHERE 1=1; --",
            "0x`) UNION SELECT * FROM private_keys--",
            "0x; UPDATE balances SET amount=999999",
            "Robert'); DROP TABLE Students;--",
        ];
        
        for attempt in sql_injection_attempts {
            let result = validator.validate_address(attempt);
            assert!(result.is_err(), "Should reject SQL injection: {}", attempt);
        }
    }

    #[test]
    fn test_address_xss_prevention() {
        let validator = InputValidator::new();
        
        let xss_attempts = vec![
            "<script>alert('xss')</script>",
            "0x<img src=x onerror=alert(1)>",
            "javascript:alert('xss')",
            "0x\"><script>alert(String.fromCharCode(88,83,83))</script>",
            "&lt;script&gt;alert('xss')&lt;/script&gt;",
        ];
        
        for attempt in xss_attempts {
            let result = validator.validate_address(attempt);
            assert!(result.is_err(), "Should reject XSS attempt: {}", attempt);
        }
    }
}

#[cfg(test)]
mod transaction_hash_validation_tests {
    use super::*;

    #[test]
    fn test_valid_transaction_hashes() {
        let validator = InputValidator::new();
        
        let valid_hashes = vec![
            // Real mainnet transaction hashes
            "0x5c504ed432cb51138bcf09aa5e8a410dd4a1e204ef84bfed1be16dfba1b22060", // First ETH tx
            "0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006", // Complex DeFi
            "0x7b944d902fd772fa5bb34f923b3b03307f8af57043b7fd7c2b101771e03cf42b", // Uniswap trade
            "0x0000000000000000000000000000000000000000000000000000000000000000", // All zeros
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", // All ones
        ];
        
        for hash in valid_hashes {
            let result = validator.validate_transaction_hash(hash);
            assert!(result.is_ok(), "Failed to validate hash: {}", hash);
        }
    }

    #[test]
    fn test_invalid_transaction_hashes() {
        let validator = InputValidator::new();
        
        let invalid_hashes = vec![
            ("", "Empty string"),
            ("0x", "Only prefix"),
            ("0x123", "Too short"),
            ("f7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006", "Missing 0x"),
            ("0xZZZZ63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c006", "Invalid hex"),
            ("0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c00", "Too short (65)"),
            ("0xf7bd63f7b61b4dc88ffb081a05d0e29b6558649802285838128c10fc9ce6c0066", "Too long (67)"),
        ];
        
        for (hash, description) in invalid_hashes {
            let result = validator.validate_transaction_hash(hash);
            assert!(result.is_err(), "Should reject {}: {}", description, hash);
        }
    }
}

#[cfg(test)]
mod numeric_validation_tests {
    use super::*;

    #[test]
    fn test_eth_value_validation() {
        let validator = InputValidator::new();
        
        // Valid values
        assert!(validator.validate_eth_value(0.0).is_ok());
        assert!(validator.validate_eth_value(0.000000000000000001).is_ok()); // 1 wei
        assert!(validator.validate_eth_value(1.0).is_ok());
        assert!(validator.validate_eth_value(1000000.0).is_ok()); // 1M ETH
        
        // Invalid values
        assert!(validator.validate_eth_value(-1.0).is_err()); // Negative
        assert!(validator.validate_eth_value(f64::INFINITY).is_err());
        assert!(validator.validate_eth_value(f64::NEG_INFINITY).is_err());
        assert!(validator.validate_eth_value(f64::NAN).is_err());
        assert!(validator.validate_eth_value(10_000_000.0).is_err()); // Over max
    }

    #[test]
    fn test_u256_validation() {
        let validator = InputValidator::new();
        
        // Valid U256 values
        assert!(validator.validate_u256("0").is_ok());
        assert!(validator.validate_u256("1").is_ok());
        assert!(validator.validate_u256("1000000000000000000").is_ok()); // 1 ETH in wei
        assert!(validator.validate_u256("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").is_ok());
        
        // Invalid values
        assert!(validator.validate_u256("-1").is_err()); // Negative
        assert!(validator.validate_u256("not a number").is_err());
        assert!(validator.validate_u256("").is_err());
        assert!(validator.validate_u256("12.34").is_ok()); // Decimals should work
    }

    #[test]
    fn test_block_range_validation() {
        let validator = InputValidator::new();
        
        // Valid ranges
        assert!(validator.validate_block_range(0, 0).is_ok());
        assert!(validator.validate_block_range(1, 100).is_ok());
        assert!(validator.validate_block_range(17_000_000, 17_100_000).is_ok());
        
        // Invalid ranges
        assert!(validator.validate_block_range(100, 1).is_err()); // Start > end
        assert!(validator.validate_block_range(0, 200_000).is_err()); // Too large range
    }
}

#[cfg(test)]
mod label_validation_tests {
    use super::*;

    #[test]
    fn test_valid_labels() {
        let validator = InputValidator::new();
        
        let valid_labels = vec![
            "Uniswap V3 Router",
            "vitalik.eth",
            "Binance Hot Wallet 7",
            "USDC_Token_Contract",
            "compound-finance-v2",
            "1inch.exchange",
            "Test Label 123",
        ];
        
        for label in valid_labels {
            let result = validator.validate_label(label);
            assert!(result.is_ok(), "Should accept label: {}", label);
            assert_eq!(result.unwrap(), label.trim());
        }
    }

    #[test]
    fn test_invalid_labels() {
        let validator = InputValidator::new();
        
        // Too long label
        let long_label = "a".repeat(300);
        assert!(validator.validate_label(&long_label).is_err());
        
        // Invalid characters
        let invalid_labels = vec![
            "Label with <script>",
            "Label'; DROP TABLE--",
            "Label\" OR \"1\"=\"1",
            "Label\nwith\nnewlines",
            "Label\twith\ttabs",
            "Label with emoji 🚀", // Depends on pattern
        ];
        
        for label in invalid_labels {
            let result = validator.validate_label(label);
            assert!(result.is_err(), "Should reject label: {}", label);
        }
    }

    #[test]
    fn test_label_sanitization() {
        let validator = InputValidator::new();
        
        // Whitespace trimming
        assert_eq!(validator.validate_label("  spaces  ").unwrap(), "spaces");
        assert_eq!(validator.validate_label("\nleading").unwrap_err().to_string().contains("invalid characters"), true);
        
        // Empty after trim
        assert_eq!(validator.validate_label("   ").unwrap(), "");
    }
}

#[cfg(test)]
mod sql_identifier_tests {
    use super::*;

    #[test]
    fn test_valid_sql_identifiers() {
        let validator = InputValidator::new();
        
        let valid_identifiers = vec![
            "users",
            "user_accounts",
            "transactions2023",
            "eth_transfers",
            "UPPERCASE",
            "CamelCase",
        ];
        
        for identifier in valid_identifiers {
            let result = validator.sanitize_sql_identifier(identifier);
            assert!(result.is_ok(), "Should accept identifier: {}", identifier);
            assert_eq!(result.unwrap(), identifier.to_lowercase());
        }
    }

    #[test]
    fn test_sql_reserved_words() {
        let validator = InputValidator::new();
        
        let reserved_words = vec![
            "SELECT", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER",
            "select", "Select", "SeLeCt", // Case variations
        ];
        
        for word in reserved_words {
            let result = validator.sanitize_sql_identifier(word);
            assert!(result.is_err(), "Should reject reserved word: {}", word);
        }
    }

    #[test]
    fn test_sql_injection_in_identifiers() {
        let validator = InputValidator::new();
        
        let injections = vec![
            "users; DROP TABLE accounts",
            "users' OR '1'='1",
            "users/*comment*/",
            "users--comment",
            "users\\",
            "users`",
            "users()",
        ];
        
        for injection in injections {
            let result = validator.sanitize_sql_identifier(injection);
            assert!(result.is_err(), "Should reject SQL injection: {}", injection);
        }
    }
}

#[cfg(test)]
mod security_detection_tests {
    use super::*;
    use validation::security;

    #[test]
    fn test_suspicious_pattern_detection() {
        // SQL injection patterns
        assert!(security::detect_suspicious_pattern("'; DROP TABLE users; --"));
        assert!(security::detect_suspicious_pattern("' OR '1'='1' --"));
        assert!(security::detect_suspicious_pattern("UNION SELECT * FROM"));
        assert!(security::detect_suspicious_pattern("'; EXEC xp_cmdshell"));
        
        // XSS patterns
        assert!(security::detect_suspicious_pattern("<script>alert('xss')</script>"));
        assert!(security::detect_suspicious_pattern("javascript:void(0)"));
        assert!(security::detect_suspicious_pattern("onerror=alert(1)"));
        
        // Path traversal
        assert!(security::detect_suspicious_pattern("../../etc/passwd"));
        assert!(security::detect_suspicious_pattern("..\\windows\\system32"));
        assert!(security::detect_suspicious_pattern("%2e%2e%2f"));
        assert!(security::detect_suspicious_pattern("%252e%252e%252f"));
        
        // Normal inputs should pass
        assert!(!security::detect_suspicious_pattern("normal transaction data"));
        assert!(!security::detect_suspicious_pattern("0x742d35Cc6634C0532925a3b844Bc9e7595f5b899"));
        assert!(!security::detect_suspicious_pattern("Transfer 1.5 ETH to alice.eth"));
    }

    #[test]
    fn test_address_redaction() {
        let addr = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f5b899").unwrap();
        let redacted = security::redact_address(&addr);
        assert_eq!(redacted, "0x742d...b899");
    }

    #[test]
    fn test_hash_for_logging() {
        let sensitive = "secret_api_key_12345";
        let hash1 = security::hash_for_logging(sensitive);
        let hash2 = security::hash_for_logging(sensitive);
        
        // Should be deterministic
        assert_eq!(hash1, hash2);
        
        // Should start with SHA256:
        assert!(hash1.starts_with("SHA256:"));
        
        // Should be truncated (not full hash)
        assert!(hash1.len() < 80);
        
        // Different inputs should have different hashes
        let hash3 = security::hash_for_logging("different_secret");
        assert_ne!(hash1, hash3);
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_unicode_handling() {
        let validator = InputValidator::new();
        
        // Unicode in various inputs
        let unicode_inputs = vec![
            ("0x742d35Cc6634C0532925a3b844Bc9e7595f5b899", "Normal ASCII"),
            ("０ｘ７４２ｄ３５Ｃｃ６６３４Ｃ０５３２９２５ａ３ｂ８４４Ｂｃ９ｅ７５９５ｆ５ｂ８９９", "Full-width"),
        ];
        
        for (input, description) in unicode_inputs {
            if description == "Normal ASCII" {
                assert!(validator.validate_address(input).is_ok());
            } else {
                assert!(validator.validate_address(input).is_err());
            }
        }
    }

    #[test]
    fn test_boundary_values() {
        let validator = InputValidator::new();
        
        // Test at boundaries
        assert!(validator.validate_depth(1).is_ok());
        assert!(validator.validate_depth(10).is_ok()); // Max
        assert!(validator.validate_depth(11).is_err()); // Over max
        assert!(validator.validate_depth(0).is_err()); // Under min
        
        assert!(validator.validate_batch_size(1).is_ok());
        assert!(validator.validate_batch_size(1000).is_ok()); // Max
        assert!(validator.validate_batch_size(1001).is_err()); // Over max
        assert!(validator.validate_batch_size(0).is_err()); // Under min
    }

    #[test]
    fn test_custom_validation_rules() {
        let custom_rules = ValidationRules {
            max_depth: 5,
            max_batch_size: 100,
            max_block_range: 1000,
            max_eth_value: 100.0,
            min_eth_value: 0.001,
            max_label_length: 50,
            label_pattern: regex::Regex::new(r"^[a-zA-Z0-9]+$").unwrap(),
        };
        
        let validator = InputValidator::with_rules(custom_rules);
        
        // Test custom limits
        assert!(validator.validate_depth(5).is_ok());
        assert!(validator.validate_depth(6).is_err());
        
        assert!(validator.validate_batch_size(100).is_ok());
        assert!(validator.validate_batch_size(101).is_err());
        
        assert!(validator.validate_eth_value(100.0).is_ok());
        assert!(validator.validate_eth_value(100.1).is_err());
        
        // Custom label pattern (alphanumeric only)
        assert!(validator.validate_label("AlphaNum123").is_ok());
        assert!(validator.validate_label("With Spaces").is_err());
        assert!(validator.validate_label("with-dashes").is_err());
    }
}

// Property-based tests
#[cfg(all(test, feature = "proptest"))]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_address_validation_never_panics(s in ".*") {
            let validator = InputValidator::new();
            let _ = validator.validate_address(&s);
        }

        #[test]
        fn test_valid_hex_addresses_accepted(hex in "[0-9a-fA-F]{40}") {
            let validator = InputValidator::new();
            let address = format!("0x{}", hex);
            assert!(validator.validate_address(&address).is_ok());
        }

        #[test]
        fn test_label_validation_never_panics(s in ".*") {
            let validator = InputValidator::new();
            let _ = validator.validate_label(&s);
        }

        #[test]
        fn test_eth_value_validation_consistency(value: f64) {
            let validator = InputValidator::new();
            let result1 = validator.validate_eth_value(value);
            let result2 = validator.validate_eth_value(value);
            // Should be deterministic
            assert_eq!(result1.is_ok(), result2.is_ok());
        }
    }
}