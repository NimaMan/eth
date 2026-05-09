#!/bin/bash
# Update alloy imports to use individual crates

echo "Updating alloy imports..."

# Update imports in all Rust files
find . -name "*.rs" -type f -not -path "./target/*" -exec sed -i 's/use alloy::primitives/use alloy_primitives/g' {} \;

# Also need to update Cargo.toml files that reference alloy
find . -name "Cargo.toml" -not -path "./target/*" -exec sed -i 's/alloy = { workspace = true }/alloy-primitives = { workspace = true }/g' {} \;

echo "Done updating imports"