.PHONY: help build run test test-all test-lib test-integration test-verbose clean fmt lint check run-guidelines package

help:
	@echo "Available targets:"
	@echo "  build            - Build the project"
	@echo "  run              - Run the project"
	@echo "  test             - Run core tests (no interchange)"
	@echo "  test-all         - Run all tests with interchange"
	@echo "  test-lib         - Run library tests with interchange"
	@echo "  test-integration - Run integration tests (editing, roundtrip, decompiler)"
	@echo "  clean            - Clean build artifacts"
	@echo "  fmt              - Format code with rustfmt"
	@echo "  lint             - Run clippy linter (with interchange)"
	@echo "  check            - Run fmt + lint + test-all"
	@echo "  run-guidelines   - Run complete validation (fmt + lint + build + test)"

build:
	cargo build --features interchange

release:
	cargo build --release --features interchange

run:
	cargo run

test:
	cargo test

test-all:
	cargo test --features interchange

test-lib:
	cargo test --features interchange --lib

test-integration:
	cargo test --test test_editing_integration --features interchange

test-verbose:
	cargo test --features interchange -- --nocapture

clean:
	cargo clean

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

lint:
	cargo clippy --all-targets --features interchange -- -D warnings

check: fmt-check lint test-all

run-guidelines:
	@echo "=== Running Complete Validation Pipeline ==="
	@echo ""
	@echo "Step 1/3: Formatting code..."
	@cargo fmt
	@echo "✓ Code formatted"
	@echo ""
	@echo "Step 2/3: Running linter (includes build)..."
	@cargo clippy --all-targets --features interchange -- -D warnings
	@echo "✓ Linting passed"
	@echo ""
	@echo "Step 3/3: Running all tests (with interchange)..."
	@cargo test --features interchange
	@echo ""
	@echo "=== ✓ All guidelines passed! ==="

package:
	@echo "Building package..."
	@cargo build --release --features interchange
	@echo "✓ Package built"
