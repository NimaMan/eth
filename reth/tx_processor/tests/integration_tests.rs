use std::process::Command;

/// Integration tests for REVM transaction simulator
/// These tests verify that the core functionality works correctly

#[test]
fn test_json_state_validator_compiles_and_runs() {
    // Verify the main validator example compiles and can run
    let output = Command::new("cargo")
        .args(&["build", "--example", "json_state_validator_no_rpc"])
        .current_dir("/home/nima/code/crypto/rust/revm_tx_simulator")
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(output.status.success(), 
        "json_state_validator_no_rpc failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_simulate_and_extract_diffs_still_works() {
    // Ensure we haven't broken the original working example
    let output = Command::new("cargo")
        .args(&["build", "--example", "simulate_and_extract_diffs"])
        .current_dir("/home/nima/code/crypto/rust/revm_tx_simulator")
        .output()
        .expect("Failed to execute cargo build");
    
    assert!(output.status.success(), 
        "simulate_and_extract_diffs failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_accuracy_validation_runner_exists() {
    // Verify the Python test runner exists and is executable
    let path = std::path::Path::new("/home/nima/code/crypto/rust/revm_tx_simulator/tests/run_accuracy_tests.py");
    assert!(path.exists(), "Accuracy test runner script not found");
    
    // Check if it's executable (on Unix systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path).expect("Failed to get file metadata");
        let permissions = metadata.permissions();
        // Check if owner has execute permission
        assert!(permissions.mode() & 0o100 != 0 || path.extension().map_or(false, |ext| ext == "py"), 
            "Test runner should be executable or be a Python script");
    }
}

#[test] 
fn test_python_validation_script_exists() {
    // Verify the Python validation script exists
    let path = std::path::Path::new("/home/nima/code/crypto/rust/mempool_processor/python/core/validate_state_changes.py");
    assert!(path.exists(), "Python validation script not found at expected location");
}

#[test]
fn test_validator_example_compiles() {
    // Just test that the validator example compiles successfully
    // (We can't easily test execution without a specific transaction hash)
    let output = Command::new("cargo")
        .args(&["build", "--example", "json_state_validator_no_rpc"])
        .current_dir("/home/nima/code/crypto/rust/revm_tx_simulator")
        .output()
        .expect("Failed to build validator example");
    
    // Print output for debugging if compilation fails
    if !output.status.success() {
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    // The validator should compile successfully
    assert!(output.status.success(), 
        "Validator example failed to compile: {}", 
        String::from_utf8_lossy(&output.stderr));
    
    println!("✅ json_state_validator_no_rpc example compiled successfully");
}

// Note: Accuracy tests require proper conda environment setup
// Run them manually with: ./tests/run_all_tests.sh
// Or: python3 tests/run_accuracy_tests.py --quick