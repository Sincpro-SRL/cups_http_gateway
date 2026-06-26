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

# ── Cross-platform release builds ─────────────────────────────────────────────

CARGO_VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
BINARY_NAME   := cups-http-gateway
DIST_DIR      := dist

$(DIST_DIR):
	mkdir -p $(DIST_DIR)

install-cross:
	@if ! command -v cross > /dev/null 2>&1; then \
		echo "Installing cross..."; \
		cargo install cross --locked; \
	else \
		echo "cross already installed: $$(cross --version)"; \
	fi

build-linux-x64:
	rustup target add x86_64-unknown-linux-gnu
	cargo build --release --target x86_64-unknown-linux-gnu

build-linux-arm64:
	rustup target add aarch64-unknown-linux-gnu
	cross build --release --target aarch64-unknown-linux-gnu

build-macos-x64:
	rustup target add x86_64-apple-darwin
	cargo build --release --target x86_64-apple-darwin

build-macos-arm64:
	rustup target add aarch64-apple-darwin
	cargo build --release --target aarch64-apple-darwin

build-windows-x64:
	rustup target add x86_64-pc-windows-msvc
	cargo build --release --target x86_64-pc-windows-msvc

package-linux-x64: $(DIST_DIR)
	cp target/x86_64-unknown-linux-gnu/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)
	tar -czf $(DIST_DIR)/$(BINARY_NAME)-linux-x86_64.tar.gz -C $(DIST_DIR) $(BINARY_NAME)
	rm $(DIST_DIR)/$(BINARY_NAME)
	echo "Packaged: $(DIST_DIR)/$(BINARY_NAME)-linux-x86_64.tar.gz"

package-linux-arm64: $(DIST_DIR)
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)
	tar -czf $(DIST_DIR)/$(BINARY_NAME)-linux-aarch64.tar.gz -C $(DIST_DIR) $(BINARY_NAME)
	rm $(DIST_DIR)/$(BINARY_NAME)
	echo "Packaged: $(DIST_DIR)/$(BINARY_NAME)-linux-aarch64.tar.gz"

package-macos-x64: $(DIST_DIR)
	cp target/x86_64-apple-darwin/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)
	tar -czf $(DIST_DIR)/$(BINARY_NAME)-macos-x86_64.tar.gz -C $(DIST_DIR) $(BINARY_NAME)
	rm $(DIST_DIR)/$(BINARY_NAME)
	echo "Packaged: $(DIST_DIR)/$(BINARY_NAME)-macos-x86_64.tar.gz"

package-macos-arm64: $(DIST_DIR)
	cp target/aarch64-apple-darwin/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)
	tar -czf $(DIST_DIR)/$(BINARY_NAME)-macos-arm64.tar.gz -C $(DIST_DIR) $(BINARY_NAME)
	rm $(DIST_DIR)/$(BINARY_NAME)
	echo "Packaged: $(DIST_DIR)/$(BINARY_NAME)-macos-arm64.tar.gz"

package-windows-x64: $(DIST_DIR)
	cp target/x86_64-pc-windows-msvc/release/$(BINARY_NAME).exe $(DIST_DIR)/$(BINARY_NAME).exe
	powershell -Command "Compress-Archive -Path $(DIST_DIR)/$(BINARY_NAME).exe -DestinationPath $(DIST_DIR)/$(BINARY_NAME)-windows-x86_64.zip -Force"
	rm $(DIST_DIR)/$(BINARY_NAME).exe
	echo "Packaged: $(DIST_DIR)/$(BINARY_NAME)-windows-x86_64.zip"

release-linux-x64: build-linux-x64 package-linux-x64
release-linux-arm64: install-cross build-linux-arm64 package-linux-arm64
release-macos-x64: build-macos-x64 package-macos-x64
release-macos-arm64: build-macos-arm64 package-macos-arm64
release-windows-x64: build-windows-x64 package-windows-x64

.PHONY: prepare-environment install init \
        format format-rust format-yaml \
        lint verify-format \
        test test-debug \
        check-lib ci audit \
        update-version build \
        publish publish-dry \
        install-cross \
        build-linux-x64 build-linux-arm64 build-macos-x64 build-macos-arm64 build-windows-x64 \
        package-linux-x64 package-linux-arm64 package-macos-x64 package-macos-arm64 package-windows-x64 \
        release-linux-x64 release-linux-arm64 release-macos-x64 release-macos-arm64 release-windows-x64
