//! MDBX version compatibility utilities
//!
//! This module provides utilities for handling MDBX version mismatches
//! and ensuring compatibility when opening Reth databases.

use std::path::Path;
use std::fs;
use tracing::{debug, warn, info};

use crate::fetch_from_reth::error::{FetchError, FetchResult};

/// MDBX error codes
pub const MDBX_VERSION_MISMATCH: i32 = -30794;
pub const MDBX_EINVAL: i32 = 22;
pub const MDBX_ENOENT: i32 = 2;
pub const MDBX_EACCESS: i32 = 13;

/// Database version information
#[derive(Debug, Clone)]
pub struct DatabaseVersion {
    /// MDBX library version
    pub mdbx_version: String,
    /// Reth version that created the database
    pub reth_version: Option<String>,
    /// Database format version
    pub db_version: Option<u32>,
}

/// Read database version information from files
pub fn read_database_version(datadir: &Path) -> FetchResult<DatabaseVersion> {
    let db_path = datadir.join("db");
    
    // Try to read database.version file
    let version_file = db_path.join("database.version");
    let db_version = if version_file.exists() {
        match fs::read_to_string(&version_file) {
            Ok(content) => {
                // Parse version from file content
                content.trim().parse::<u32>().ok()
            }
            Err(e) => {
                debug!("Failed to read database.version: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    // Try to read MDBX version from lock file or metadata
    let lock_file = db_path.join("mdbx.lck");
    let mdbx_version = if lock_file.exists() {
        // Lock file exists, database might be in use
        warn!("MDBX lock file exists at {:?}, database may be in use", lock_file);
        "unknown (lock file present)".to_string()
    } else {
        // Default version string
        "unknown".to_string()
    };
    
    Ok(DatabaseVersion {
        mdbx_version,
        reth_version: None,  // Could be read from additional metadata
        db_version,
    })
}

/// Check if the error is a version mismatch
pub fn is_version_mismatch_error(error: &str) -> bool {
    error.contains("version mismatch") || 
    error.contains("-30794") ||
    error.contains("MDB_VERSION_MISMATCH") ||
    error.contains("Database environment version mismatch")
}

/// Extract error code from error message
pub fn extract_error_code(error: &str) -> Option<i32> {
    // Try to extract error code from patterns like "error code: -30794"
    if let Some(pos) = error.find("error code:") {
        let after_colon = &error[pos + 11..].trim_start();
        // Find the end of the number
        let end = after_colon.find(|c: char| !c.is_numeric() && c != '-')
            .unwrap_or(after_colon.len());
        if end > 0 {
            return after_colon[..end].parse().ok();
        }
    }
    
    // Try to extract from patterns like "(-30794)"
    if let Some(start) = error.find('(') {
        let code_str = &error[start + 1..];
        if let Some(end) = code_str.find(')') {
            let potential_code = code_str[..end].trim();
            if potential_code.starts_with('-') || potential_code.chars().all(|c| c.is_numeric()) {
                return potential_code.parse().ok();
            }
        }
    }
    
    None
}

/// Get recommended MDBX flags for compatibility mode
pub fn get_compatibility_flags() -> u32 {
    // Based on libmdbx documentation:
    // MDBX_RDONLY = 0x20000
    // MDBX_NOSUBDIR = 0x4000
    // MDBX_NORDAHEAD = 0x800000
    // MDBX_NOMEMINIT = 0x1000000
    
    let mut flags = 0x20000; // MDBX_RDONLY
    
    // Add flags that help with compatibility
    flags |= 0x800000;  // MDBX_NORDAHEAD - don't readahead (helps with lock issues)
    flags |= 0x1000000; // MDBX_NOMEMINIT - don't initialize malloc'd memory
    
    flags
}

/// Check if we can bypass version check safely
pub fn can_bypass_version_check(expected: &str, actual: &str) -> bool {
    // Parse version numbers if possible
    let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            if let (Ok(major), Ok(minor), Ok(patch)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>()
            ) {
                return Some((major, minor, patch));
            }
        }
        None
    };
    
    if let (Some(exp), Some(act)) = (parse_version(expected), parse_version(actual)) {
        // Allow minor version differences within same major version
        if exp.0 == act.0 {  // Same major version
            if exp.0 == 0 {
                // For 0.x.y versions, the minor version is effectively the major version
                if exp.1 == act.1 {
                    info!("MDBX versions are compatible: {} vs {}", expected, actual);
                    return true;
                }
            } else {
                info!("MDBX versions have same major version: {} vs {}", expected, actual);
                return true;
            }
        }
    }
    
    false
}

/// Remove lock file if it exists and we're in force mode
pub fn remove_lock_file_if_safe(db_path: &Path, force: bool) -> FetchResult<()> {
    let lock_file = db_path.join("mdbx.lck");
    
    if lock_file.exists() && force {
        warn!("Removing MDBX lock file at {:?} (force mode enabled)", lock_file);
        match fs::remove_file(&lock_file) {
            Ok(_) => {
                info!("Successfully removed MDBX lock file");
                Ok(())
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    Err(FetchError::ConfigError(
                        "Cannot remove lock file: permission denied. Run with appropriate permissions or ensure database is not in use".to_string()
                    ))
                } else {
                    Err(FetchError::IoError(format!("Failed to remove lock file: {}", e)))
                }
            }
        }
    } else {
        Ok(())
    }
}

/// Log version mismatch details for debugging
pub fn log_version_mismatch_details(error: &str, datadir: &Path) {
    warn!("MDBX version mismatch error: {}", error);
    
    if let Ok(version) = read_database_version(datadir) {
        info!("Database version info: {:?}", version);
    }
    
    if let Some(code) = extract_error_code(error) {
        info!("Error code: {}", code);
        match code {
            MDBX_VERSION_MISMATCH => info!("This is a version mismatch error"),
            MDBX_EINVAL => info!("Invalid argument - might be incompatible flags"),
            MDBX_ENOENT => info!("File not found - check database path"),
            MDBX_EACCESS => info!("Access denied - check permissions"),
            _ => info!("Unknown error code"),
        }
    }
    
    info!("Try the following solutions:");
    info!("1. Use compatibility mode: .with_compatibility_mode(CompatibilityMode::Compatible)");
    info!("2. Force open (risky): .with_force_open(true)");
    info!("3. Check if Reth is running and using the database");
    info!("4. Verify database path and permissions");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_version_mismatch_error() {
        assert!(is_version_mismatch_error("MDB_VERSION_MISMATCH: Database environment version mismatch"));
        assert!(is_version_mismatch_error("error code: -30794"));
        assert!(is_version_mismatch_error("version mismatch detected"));
        assert!(!is_version_mismatch_error("file not found"));
    }
    
    #[test]
    fn test_extract_error_code() {
        assert_eq!(extract_error_code("error code: -30794"), Some(-30794));
        assert_eq!(extract_error_code("failed (-30794)"), Some(-30794));
        assert_eq!(extract_error_code("error code: 22"), Some(22));
        assert_eq!(extract_error_code("no code here"), None);
    }
    
    #[test]
    fn test_can_bypass_version_check() {
        assert!(can_bypass_version_check("0.2.0", "0.2.1"));
        assert!(can_bypass_version_check("0.2.0", "0.2.5"));
        assert!(!can_bypass_version_check("0.2.0", "0.3.0"));
        assert!(!can_bypass_version_check("0.2.0", "1.0.0"));
    }
    
    #[test]
    fn test_compatibility_flags() {
        let flags = get_compatibility_flags();
        assert!(flags & 0x20000 != 0); // MDBX_RDONLY
        assert!(flags & 0x800000 != 0); // MDBX_NORDAHEAD
    }
}