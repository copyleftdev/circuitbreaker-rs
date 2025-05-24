.PHONY: build test bench clean doc clippy fmt fmt-check help check publish-dry-run publish ci install-tools

# Default target when just running `make`
.DEFAULT_GOAL := help

# Colorful output
CYAN=\033[0;36m
GREEN=\033[0;32m
YELLOW=\033[0;33m
RED=\033[0;31m
NC=\033[0m # No Color

help: ## Display this help
	@echo "$(CYAN)CircuitBreaker-rs Makefile$(NC)"
	@echo "$(CYAN)===================$(NC)"
	@echo
	@echo "$(YELLOW)Usage:$(NC)"
	@echo "  make $(GREEN)<target>$(NC)"
	@echo
	@echo "$(YELLOW)Targets:$(NC)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2}'

build: ## Build the project in debug mode
	@echo "$(CYAN)Building CircuitBreaker-rs (Debug)...$(NC)"
	cargo build

release: ## Build the project in release mode
	@echo "$(CYAN)Building CircuitBreaker-rs (Release)...$(NC)"
	cargo build --release

test: ## Run tests
	@echo "$(CYAN)Running tests...$(NC)"
	cargo test --all-features

test-all: ## Run tests with all features and all targets
	@echo "$(CYAN)Running comprehensive tests...$(NC)"
	cargo test --all-features --all-targets

bench: ## Run benchmarks
	@echo "$(CYAN)Running benchmarks...$(NC)"
	cargo bench

bench-save: ## Run benchmarks and save results to bench-results directory
	@echo "$(CYAN)Running benchmarks with saved results...$(NC)"
	@mkdir -p bench-results
	@cargo bench -- --output-format=json | tee bench-results/$(shell date +%Y-%m-%d-%H-%M-%S).json

doc: ## Generate documentation
	@echo "$(CYAN)Generating documentation...$(NC)"
	cargo doc --no-deps
	@echo "$(GREEN)Documentation generated in target/doc/$(NC)"

doc-open: ## Generate documentation and open in browser
	@echo "$(CYAN)Generating documentation and opening in browser...$(NC)"
	cargo doc --no-deps --open

clippy: ## Run Clippy lints
	@echo "$(CYAN)Running Clippy...$(NC)"
	cargo clippy --all-targets --all-features -- -D warnings

clippy-fix: ## Run Clippy and apply automatic fixes
	@echo "$(CYAN)Running Clippy with automatic fixes...$(NC)"
	cargo clippy --all-targets --all-features --fix --allow-no-vcs -- -D warnings

fmt: ## Format code with rustfmt
	@echo "$(CYAN)Formatting code...$(NC)"
	cargo fmt

fmt-check: ## Check if code is formatted
	@echo "$(CYAN)Checking code formatting...$(NC)"
	cargo fmt -- --check

clean: ## Clean build artifacts
	@echo "$(CYAN)Cleaning build artifacts...$(NC)"
	cargo clean

check: ## Run cargo check
	@echo "$(CYAN)Running cargo check...$(NC)"
	cargo check --all-targets --all-features

publish-dry-run: ## Perform a dry run of crates.io publishing
	@echo "$(CYAN)Performing publish dry run...$(NC)"
	cargo publish --dry-run

publish: ## Publish to crates.io (requires authentication)
	@echo "$(RED)Publishing to crates.io...$(NC)"
	@echo "$(YELLOW)Are you sure? [y/N]$(NC)" && read ans && [ $${ans:-N} = y ]
	cargo publish

ci: fmt-check clippy test ## Run CI checks (formatting, linting, tests)
	@echo "$(GREEN)All CI checks passed!$(NC)"

install-tools: ## Install development tools
	@echo "$(CYAN)Installing development tools...$(NC)"
	rustup component add clippy rustfmt
	@echo "$(CYAN)Installing cargo-audit and cargo-outdated...$(NC)"
	cargo install cargo-audit cargo-outdated
	@echo "$(CYAN)Installing cargo-tarpaulin for code coverage...$(NC)"
	cargo install cargo-tarpaulin

update-deps: ## Update dependencies
	@echo "$(CYAN)Updating dependencies...$(NC)"
	cargo update

audit: ## Run security audit on dependencies
	@echo "$(CYAN)Running security audit...$(NC)"
	@if command -v cargo-audit > /dev/null; then \
		cargo audit; \
	else \
		echo "$(RED)cargo-audit not installed. Run 'make install-tools' first.$(NC)"; \
		exit 1; \
	fi

outdated: ## Check for outdated dependencies
	@echo "$(CYAN)Checking for outdated dependencies...$(NC)"
	@if command -v cargo-outdated > /dev/null; then \
		cargo outdated; \
	else \
		echo "$(RED)cargo-outdated not installed. Run 'make install-tools' first.$(NC)"; \
		exit 1; \
	fi

coverage: ## Run code coverage analysis (requires cargo-tarpaulin)
	@echo "$(CYAN)Running code coverage analysis...$(NC)"
	@if command -v cargo-tarpaulin > /dev/null; then \
		cargo tarpaulin --all-features --out Xml; \
	else \
		echo "$(RED)cargo-tarpaulin not installed. Run 'make install-tools' first.$(NC)"; \
		exit 1; \
	fi

# Custom target for full quality check
quality: fmt clippy test doc ## Run full quality check suite
	@echo "$(GREEN)Quality check passed!$(NC)"
