.PHONY: help build run test clean fmt lint check run-guidelines

help:
	@echo "Available targets:"
	@echo "  build          - Build the project"
	@echo "  run            - Run the project"
	@echo "  test           - Run tests"
	@echo "  clean          - Clean build artifacts"
	@echo "  fmt            - Format code with rustfmt"
	@echo "  lint           - Run clippy linter"
	@echo "  check          - Run fmt + lint + test"
	@echo "  run-guidelines - Run complete validation (fmt + lint + build + test)"

build:
	cargo build

release:
	cargo build --release

run:
	cargo run

test:
	cargo test

test-verbose:
	cargo test -- --nocapture

clean:
	cargo clean

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

lint:
	cargo clippy --all-targets -- -D warnings

check: fmt-check lint test

run-guidelines:
	@echo "=== Running Complete Validation Pipeline ==="
	@echo ""
	@echo "Step 1/3: Formatting code..."
	@cargo fmt
	@echo "✓ Code formatted"
	@echo ""
	@echo "Step 2/3: Running linter (includes build)..."
	@cargo clippy --all-targets -- -D warnings
	@echo "✓ Linting passed"
	@echo ""
	@echo "Step 3/3: Running tests..."
	@cargo test
	@echo ""
	@echo "=== ✓ All guidelines passed! ==="
