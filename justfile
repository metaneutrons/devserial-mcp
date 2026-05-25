default: check

# Build debug
build:
    cargo build

# Build release
release:
    cargo build --release

# Run all tests
test:
    cargo test

# Run unit tests only (fast)
test-unit:
    cargo test --lib

# Run full CI check (fmt + clippy + test)
check:
    cargo fmt -- --check
    cargo clippy -- -D warnings
    cargo test

# Format code
fmt:
    cargo fmt

# Lint with clippy
lint:
    cargo clippy -- -D warnings

# Run the MCP server
run:
    cargo run

# Security audit
audit:
    cargo audit

# Clean build artifacts
clean:
    cargo clean

# Setup git hooks
setup:
    git config core.hooksPath .githooks
    @echo "Git hooks configured."
