NIGHTLY_RUST_VERSION = nightly
TAPLO_CLI_VERSION = 0.10.0

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    SED := $(shell command -v gsed 2>/dev/null)
    ifeq ($(SED),)
        $(error GNU sed (gsed) not found on macOS. \
			Install with: brew install gnu-sed)
    endif
else
    SED := sed
endif

.PHONY: help
help: ## Ask for help!
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | \
		awk 'BEGIN {FS = ":.*?## "}; \
		{printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

.PHONY: build
build: ## Build the project in debug mode
	cargo build

.PHONY: build-release
build-release: ## Build the project in release mode
	cargo build --release

.PHONY: check
check: ## Check code for compilation errors
	cargo check

.PHONY: check-format
check-format: ## Check code formatting
	cargo +$(NIGHTLY_RUST_VERSION) fmt --all -- --check
	taplo fmt --check

.PHONY: format
format: ## Format code
	cargo +$(NIGHTLY_RUST_VERSION) fmt --all
	taplo fmt

.PHONY: lint
lint: ## Run linter
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: test
test: ## Run tests
	cargo test

.PHONY: clean
clean: ## Clean build artifacts
	cargo clean

.PHONY: setup
setup: setup-taplo ## Setup development environment
	rustup component add rustfmt clippy
	rustup component add --toolchain $(NIGHTLY_RUST_VERSION) rustfmt

.PHONY: setup-taplo
setup-taplo: ## Install taplo TOML formatter
	@if taplo --version 2>/dev/null | grep -q $(TAPLO_CLI_VERSION); then \
		echo "taplo $(TAPLO_CLI_VERSION) already installed"; \
	else \
		cargo install taplo-cli --version $(TAPLO_CLI_VERSION) --force; \
	fi

.PHONY: run
run: ## Run the WebSocket listener
	cargo run

.PHONY: fix-trailing-whitespace
fix-trailing-whitespace: ## Remove trailing whitespaces from all files
	@echo "Removing trailing whitespaces from all files..."
	@find . -type f \( \
		-name "*.rs" -o -name "*.toml" -o -name "*.md" -o -name "*.yaml" \
		-o -name "*.yml" -o -name "*.ts" -o -name "*.tsx" \
		-o -name "*.js" -o -name "*.jsx" -o -name "*.sh" \
		-o -name "*.py" -o -name "*.go" -o -name "*.c" -o -name "*.h" \
		-o -name "*.cpp" -o -name "*.hpp" -o -name "*.json" \) \
		-not -path "./target/*" \
		-not -path "./node_modules/*" \
		-not -path "./.git/*" \
		-not -path "./dist/*" \
		-not -path "./build/*" \
		-exec sh -c \
			'echo "Processing: $$1"; $(SED) -i -e "s/[[:space:]]*$$//" "$$1"' \
			_ {} \; && \
		echo "Trailing whitespaces removed."

.PHONY: check-trailing-whitespace
check-trailing-whitespace: ## Check for trailing whitespaces in source files
	@echo "Checking for trailing whitespaces..."
	@files_with_trailing_ws=$$(find . -type f \( \
		-name "*.rs" -o -name "*.toml" -o -name "*.md" -o -name "*.yaml" \
		-o -name "*.yml" -o -name "*.ts" -o -name "*.tsx" \
		-o -name "*.js" -o -name "*.jsx" -o -name "*.sh" \
		-o -name "*.py" -o -name "*.go" -o -name "*.c" -o -name "*.h" \
		-o -name "*.cpp" -o -name "*.hpp" -o -name "*.json" \) \
		-not -path "./target/*" \
		-not -path "./node_modules/*" \
		-not -path "./.git/*" \
		-not -path "./dist/*" \
		-not -path "./build/*" \
		-exec grep -l '[[:space:]]$$' {} + 2>/dev/null || true); \
	if [ -n "$$files_with_trailing_ws" ]; then \
		echo "Files with trailing whitespaces found:"; \
		echo "$$files_with_trailing_ws" | sed 's/^/  /'; \
		echo ""; \
		echo "Run 'make fix-trailing-whitespace' to fix automatically."; \
		exit 1; \
	else \
		echo "No trailing whitespaces found."; \
	fi
