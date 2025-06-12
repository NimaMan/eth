//! Tests for MDBX version compatibility handling

#[cfg(test)]
mod tests {
    use crate::fetch_from_reth::{
        RethDataConfig, CompatibilityMode, RethDatabaseProvider,
        compatibility::*, error::FetchError,
    };
    use std::path::PathBuf;
    
    #[test]
    fn test_is_version_mismatch_error() {
        // Test various error messages
        assert!(is_version_mismatch_error("MDB_VERSION_MISMATCH: Database environment version mismatch"));
        assert!(is_version_mismatch_error("error code: -30794"));
        assert!(is_version_mismatch_error("version mismatch detected"));
        assert!(is_version_mismatch_error("Database environment version mismatch"));
        
        // Should not match these
        assert!(!is_version_mismatch_error("file not found"));
        assert!(!is_version_mismatch_error("permission denied"));
        assert!(!is_version_mismatch_error("generic database error"));
    }
    
    #[test]
    fn test_extract_error_code() {
        assert_eq!(extract_error_code("error code: -30794"), Some(-30794));
        assert_eq!(extract_error_code("failed with error code: 22"), Some(22));
        assert_eq!(extract_error_code("MDBX error (-30794)"), Some(-30794));
        assert_eq!(extract_error_code("error (-123) occurred"), Some(-123));
        
        // Should not extract from these
        assert_eq!(extract_error_code("no error code here"), None);
        assert_eq!(extract_error_code("error: something went wrong"), None);
    }
    
    #[test]
    fn test_can_bypass_version_check() {
        // Same major version - should allow
        assert!(can_bypass_version_check("0.2.0", "0.2.1"));
        assert!(can_bypass_version_check("0.2.0", "0.2.5"));
        assert!(can_bypass_version_check("1.0.0", "1.0.1"));
        assert!(can_bypass_version_check("1.2.3", "1.2.4"));
        
        // Different major version - should not allow
        assert!(!can_bypass_version_check("0.2.0", "0.3.0"));
        assert!(!can_bypass_version_check("0.2.0", "1.0.0"));
        assert!(!can_bypass_version_check("1.0.0", "2.0.0"));
        
        // Invalid versions - should not allow
        assert!(!can_bypass_version_check("invalid", "0.2.0"));
        assert!(!can_bypass_version_check("0.2.0", "invalid"));
        assert!(!can_bypass_version_check("0.2", "0.2.0")); // Missing patch
    }
    
    #[test]
    fn test_compatibility_flags() {
        let flags = get_compatibility_flags();
        
        // Check that read-only flag is set
        assert!(flags & 0x20000 != 0); // MDBX_RDONLY
        
        // Check that no-readahead flag is set
        assert!(flags & 0x800000 != 0); // MDBX_NORDAHEAD
        
        // Check that no-meminit flag is set
        assert!(flags & 0x1000000 != 0); // MDBX_NOMEMINIT
    }
    
    #[test]
    fn test_compatibility_mode_config() {
        let config = RethDataConfig::new("/tmp/test")
            .with_compatibility_mode(CompatibilityMode::Compatible);
        
        assert_eq!(config.compatibility_mode, CompatibilityMode::Compatible);
        
        let config = RethDataConfig::new("/tmp/test")
            .with_compatibility_mode(CompatibilityMode::Force)
            .with_force_open(true);
        
        assert_eq!(config.compatibility_mode, CompatibilityMode::Force);
        assert!(config.force_open);
    }
    
    #[test]
    fn test_version_mismatch_error_creation() {
        let error = FetchError::version_mismatch("0.2.0", "0.3.0", Some(-30794));
        
        match error {
            FetchError::VersionMismatch { expected, actual, code } => {
                assert_eq!(expected, "0.2.0");
                assert_eq!(actual, "0.3.0");
                assert_eq!(code, Some(-30794));
            }
            _ => panic!("Expected VersionMismatch error"),
        }
        
        // Test display
        let error_str = error.to_string();
        assert!(error_str.contains("version mismatch"));
        assert!(error_str.contains("0.2.0"));
        assert!(error_str.contains("0.3.0"));
        assert!(error_str.contains("-30794"));
    }
    
    #[test]
    fn test_error_category() {
        use crate::fetch_from_reth::error::ErrorCategory;
        
        let error = FetchError::version_mismatch("0.2.0", "0.3.0", None);
        assert_eq!(error.category(), ErrorCategory::Permanent);
        assert!(!error.is_retryable());
    }
    
    // Integration test (requires actual database)
    #[test]
    #[ignore] // Ignore by default as it requires a real Reth database
    fn test_version_mismatch_handling_integration() {
        let datadir = std::env::var("RETH_DATADIR")
            .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
        
        // Try different compatibility modes
        let configs = vec![
            RethDataConfig::new(&datadir)
                .with_compatibility_mode(CompatibilityMode::Strict),
            RethDataConfig::new(&datadir)
                .with_compatibility_mode(CompatibilityMode::Compatible),
            RethDataConfig::new(&datadir)
                .with_compatibility_mode(CompatibilityMode::Auto),
        ];
        
        for config in configs {
            println!("Testing with mode: {:?}", config.compatibility_mode);
            
            match RethDatabaseProvider::with_config(config) {
                Ok(_provider) => {
                    println!("Successfully opened database");
                }
                Err(e) => {
                    println!("Failed to open database: {}", e);
                    if let FetchError::VersionMismatch { .. } = e {
                        println!("Detected version mismatch as expected");
                    }
                }
            }
        }
    }
}