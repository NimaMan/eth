use std::{
    fs,
    path::{Path, PathBuf},
};

const FORBIDDEN_PRODUCTION_CONSTRUCTORS: &[&str] = &[
    "StrategyDecision::SubmitOrder(OrderIntent",
    "Ok(StrategyDecision::SubmitOrder(",
    "return Ok(StrategyDecision::SubmitOrder(",
    "Ok(StrategyDecision::Hold)",
    "return Ok(StrategyDecision::Hold)",
];

#[test]
fn production_strategies_use_reasoned_decision_constructors() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    scan_rust_files(&src, &mut violations);

    assert!(
        violations.is_empty(),
        "strategy decisions must use StrategyDecision::submit_order(..., reason) or StrategyDecision::hold(reason):\n{}",
        violations.join("\n")
    );
}

fn scan_rust_files(path: &Path, violations: &mut Vec<String>) {
    let entries = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));

    for entry in entries {
        let entry = entry.expect("directory entry");
        let path = entry.path();
        if path.is_dir() {
            scan_rust_files(&path, violations);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for pattern in FORBIDDEN_PRODUCTION_CONSTRUCTORS {
            if content.contains(pattern) {
                violations.push(format!("{} contains `{pattern}`", path.display()));
            }
        }
    }
}
