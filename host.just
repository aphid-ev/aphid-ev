# Host workspace (root Cargo.toml): shared crates, the CAN/USB host library,
# the desktop app and build tools. Everything here builds for the desktop.

# Check formatting and lints
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# Run all host tests, including the shared no_std crates
test:
    cargo test --workspace

# Remove the host workspace's target directory
clean:
    cargo clean

# What CI runs for the host workspace
ci: check test
