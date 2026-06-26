.SILENT: prepare-environment format-yaml

# ── Setup ─────────────────────────────────────────────────────────────────────

prepare-environment:
	echo "Installing native Rust components (fmt and clippy)..."
	rustup component add rustfmt clippy
	echo "Installing cargo-edit for automatic versioning..."
	cargo install cargo-edit --locked
	echo "Installing prettier for YAML files..."
	@if ! command -v npm > /dev/null; then \
		echo "ERROR: npm is required for prettier. Please install node/npm first."; \
		exit 1; \
	fi
	npm install -g prettier
	echo "Configuring pre-commit framework..."
	@if ! command -v pre-commit > /dev/null; then \
		echo "pre-commit not found. Installing via pipx..."; \
		pipx install pre-commit; \
	else \
		echo "pre-commit is already installed globally."; \
	fi
	pre-commit install
	echo "✅ Environment ready!"

install:
	cargo fetch

init: prepare-environment install

# ── Formatting ────────────────────────────────────────────────────────────────

format-yaml:
	@if command -v prettier > /dev/null; then \
		echo "Formatting YAML files with prettier..."; \
		prettier --write "**/*.yml" "**/*.yaml"; \
	else \
		echo "prettier not found. Run: make prepare-environment"; \
	fi

format-rust:
	echo "Formatting Rust code..."
	cargo fmt --all

format: format-rust format-yaml

# ── Linting & Verification ────────────────────────────────────────────────────

lint:
	echo "Running Clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

verify-format:
	echo "Checking format compliance..."
	make format
	make lint

# ── Testing ───────────────────────────────────────────────────────────────────

test:
	cargo test --all-features

test-debug:
	cargo test --all-features -- --nocapture

# ── Guard rails ───────────────────────────────────────────────────────────────

# Validate that the library compiles without the http-server feature
check-lib:
	echo "Checking library build (no http-server feature)..."
	cargo check --no-default-features

# Run every check that CI runs, locally
ci: verify-format check-lib test audit

# Security audit against the RustSec advisory database
audit:
	@if ! command -v cargo-audit > /dev/null; then \
		echo "Installing cargo-audit..."; \
		cargo install cargo-audit --locked; \
	fi
	cargo audit

# ── Version management ────────────────────────────────────────────────────────

update-version:
ifndef VERSION
	$(error VERSION is required. Usage: make update-version VERSION=1.2.3)
endif
	@if ! cargo set-version --help > /dev/null 2>&1; then \
		echo "Installing cargo-edit..."; \
		cargo install cargo-edit --locked; \
	fi
	@echo "Updating Cargo.toml version to $(VERSION)..."
	cargo set-version $(VERSION)
	@echo "Version updated successfully"

# ── Build ─────────────────────────────────────────────────────────────────────

build:
	echo "Building optimized release binary..."
	cargo build --release

# cargo publish reads CARGO_REGISTRY_TOKEN from the environment automatically
publish:
	echo "Publishing to crates.io..."
	cargo publish

publish-dry:
	echo "Dry-run publish (no upload)..."
	cargo publish --dry-run

.PHONY: prepare-environment install init \
        format format-rust format-yaml \
        lint verify-format \
        test test-debug \
        check-lib ci audit \
        update-version build \
        publish publish-dry
