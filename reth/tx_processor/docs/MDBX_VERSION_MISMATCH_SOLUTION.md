# MDBX Version Mismatch Solution

## Problem

When accessing a Reth database, you may encounter MDBX version mismatch errors (error code -30794). This happens when:
- The MDBX library version used by your application differs from the version that created the database
- Different Reth versions use different MDBX versions
- Build options differ between environments (MDBX_LOCKING, page size, etc.)

## Solution Implemented

### 1. **Error Detection and Handling**

Added `VersionMismatch` error type to `FetchError`:
```rust
VersionMismatch {
    expected: String,
    actual: String,
    code: Option<i32>,
}
```

### 2. **Compatibility Modes**

Added `CompatibilityMode` enum with four strategies:
- **Strict**: Fail on any version mismatch (safest)
- **Compatible**: Try to open with compatibility flags
- **Force**: Remove lock files and force open (risky)
- **Auto**: Try multiple approaches in sequence (default)

### 3. **Configuration Options**

Extended `RethDataConfig` with:
```rust
pub compatibility_mode: CompatibilityMode,
pub force_open: bool,
pub mdbx_flags: Option<u32>,
```

### 4. **Compatibility Module**

Created `src/fetch_from_reth/compatibility.rs` with utilities:
- `is_version_mismatch_error()` - Detect version mismatch errors
- `extract_error_code()` - Extract error codes from messages
- `can_bypass_version_check()` - Check if versions are compatible
- `read_database_version()` - Read database version info
- `remove_lock_file_if_safe()` - Remove lock files when safe
- `log_version_mismatch_details()` - Detailed logging for debugging

### 5. **Provider Implementation**

Modified `RethDatabaseProvider::with_config()` to:
- Try different opening strategies based on compatibility mode
- Log version information for debugging
- Provide clear error messages with solutions
- Handle lock file removal when appropriate

## Usage Examples

### Basic Usage (Auto Mode)
```rust
// Automatically tries multiple approaches
let provider = RethDatabaseProvider::new("/path/to/reth/datadir")?;
```

### Compatibility Mode
```rust
let config = RethDataConfig::new("/path/to/reth/datadir")
    .with_compatibility_mode(CompatibilityMode::Compatible);
let provider = RethDatabaseProvider::with_config(config)?;
```

### Force Mode (Use with Caution)
```rust
let config = RethDataConfig::new("/path/to/reth/datadir")
    .with_compatibility_mode(CompatibilityMode::Force)
    .with_force_open(true);
let provider = RethDatabaseProvider::with_config(config)?;
```

### Fallback Strategy
```rust
fn open_database_with_fallback(datadir: &str) -> Result<RethDatabaseProvider, FetchError> {
    // Try normal mode
    if let Ok(provider) = RethDatabaseProvider::new(datadir) {
        return Ok(provider);
    }
    
    // Try compatibility mode
    let config = RethDataConfig::new(datadir)
        .with_compatibility_mode(CompatibilityMode::Compatible);
    
    if let Ok(provider) = RethDatabaseProvider::with_config(config) {
        return Ok(provider);
    }
    
    // Last resort - force mode
    let config = RethDataConfig::new(datadir)
        .with_compatibility_mode(CompatibilityMode::Force)
        .with_force_open(true);
    
    RethDatabaseProvider::with_config(config)
}
```

## Troubleshooting

If you encounter version mismatch errors:

1. **Check if Reth is running**: `ps aux | grep reth`
2. **Stop Reth if needed**: `killall reth`
3. **Check database version**: The solution logs version info automatically
4. **Remove lock file** (if safe): `rm ~/.local/share/reth/mainnet/db/mdbx.lck`
5. **Use matching Reth version**: Ensure your Reth version matches the database
6. **Rebuild database**: As last resort, resync with current Reth version

## Example Program

See `src/fetch_from_reth/examples/handle_version_mismatch.rs` for a complete example that:
- Demonstrates all compatibility modes
- Shows error handling patterns
- Provides troubleshooting guidance
- Tests database access with fallback strategies

## Testing

Run tests with:
```bash
cargo test fetch_from_reth::tests::compatibility_tests

# Run the example
cargo run --bin fetch_from_reth_handle_version_mismatch
```

## Limitations

- Reth's API doesn't expose direct MDBX flag control, limiting some compatibility options
- Force mode may lead to inconsistent reads if the database is actively being written
- Version compatibility is best-effort and may not work for all version combinations
- Always prefer using a matching Reth version over compatibility modes

## Future Improvements

1. Add automatic Reth version detection
2. Implement retry logic with exponential backoff
3. Add database repair/recovery options
4. Create migration tools for version upgrades
5. Add telemetry for version mismatch incidents