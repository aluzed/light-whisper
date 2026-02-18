SHELL := /bin/bash

# Paths
APP_NAME   := Light Whisper
BUNDLE_DIR := src-tauri/target/release/bundle
DEBUG_DIR  := src-tauri/target/debug/bundle
INSTALL_DIR := /Applications

# Python venv for icon generation
VENV       := .venv
PYTHON     := $(VENV)/bin/python3

# ─── Main targets ───

.PHONY: dev build install uninstall icons clean help

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

dev: icons ## Run in development mode (hot reload)
	cargo tauri dev

build: icons ## Build production release (.app + .dmg)
	cargo tauri build

build-debug: icons ## Build debug version (faster)
	cargo tauri build --debug

install: build ## Build and install to /Applications
	@echo "Installing $(APP_NAME) to $(INSTALL_DIR)..."
	@rm -rf "$(INSTALL_DIR)/$(APP_NAME).app"
	@cp -R "$(BUNDLE_DIR)/macos/$(APP_NAME).app" "$(INSTALL_DIR)/"
	@echo "Installed. Launch from Applications or Spotlight."

uninstall: ## Remove from /Applications and clean user data
	@echo "Removing $(APP_NAME)..."
	@rm -rf "$(INSTALL_DIR)/$(APP_NAME).app"
	@echo "Removed from $(INSTALL_DIR)."
	@echo "User data in ~/lightwhisper/ was kept. Remove manually if needed."

# ─── Icons ───

icons: $(VENV) ## Generate all icons from SVG sources
	@$(PYTHON) scripts/generate-icons.py

$(VENV): scripts/generate-icons.py
	@echo "Setting up Python venv for icon generation..."
	@python3 -m venv $(VENV)
	@$(VENV)/bin/pip install -q cairosvg Pillow
	@touch $(VENV)

# ─── Utilities ───

check: ## Type-check Rust code
	cd src-tauri && cargo check

clippy: ## Run Rust linter
	cd src-tauri && cargo clippy

clean: ## Remove build artifacts
	cd src-tauri && cargo clean
	@rm -rf $(VENV)
	@echo "Cleaned."
