.PHONY: help package package-all test check doctor validate

help: ## Show this help message
	@if command -v just >/dev/null 2>&1; then \
		just --list; \
	else \
		grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-18s\033[0m %s\n", $$1, $$2}'; \
	fi

package: ## Package an action by name, e.g. `make package opencode-usage` or `make package ACTION=opencode-usage`
	@ACTION="$(or $(ACTION),$(action),$(filter-out package,$(MAKECMDGOALS)))"; \
	if [ -z "$$ACTION" ]; then \
		echo "Usage: make package <action-name> (or make package ACTION=<action-name>)"; \
		echo "Available actions:"; \
		ls -1 actions; \
		exit 1; \
	fi; \
	if command -v just >/dev/null 2>&1; then \
		just package "$$ACTION"; \
	else \
		TARGET_DIR="actions/$$ACTION"; \
		if [ ! -d "$$TARGET_DIR" ]; then \
			echo "Error: Action '$$ACTION' not found in actions/."; \
			exit 1; \
		fi; \
		if [ -x "$$TARGET_DIR/scripts/package.sh" ]; then \
			(cd "$$TARGET_DIR" && ./scripts/package.sh); \
		elif [ -f "$$TARGET_DIR/scripts/package.sh" ]; then \
			(cd "$$TARGET_DIR" && sh ./scripts/package.sh); \
		fi; \
	fi

package-all: ## Package all actions under actions/
	@if command -v just >/dev/null 2>&1; then \
		just package-all; \
	else \
		for dir in actions/*; do \
			if [ -d "$$dir" ]; then \
				$(MAKE) package ACTION="$$(basename "$$dir")"; \
			fi; \
		done; \
	fi

test: ## Run tests for all actions or specific action (e.g. `make test ACTION=opencode-usage`)
	@if command -v just >/dev/null 2>&1; then \
		just test "$(or $(ACTION),$(action),$(filter-out test,$(MAKECMDGOALS)))"; \
	else \
		ACTION="$(or $(ACTION),$(action),$(filter-out test,$(MAKECMDGOALS)))"; \
		if [ -n "$$ACTION" ]; then \
			cargo test --manifest-path "actions/$$ACTION/Cargo.toml"; \
		else \
			for dir in actions/*; do \
				if [ -f "$$dir/Cargo.toml" ]; then \
					cargo test --manifest-path "$$dir/Cargo.toml"; \
				fi; \
			done; \
		fi; \
	fi

check: ## Run clippy and format checks for all actions or specific action
	@if command -v just >/dev/null 2>&1; then \
		just check "$(or $(ACTION),$(action),$(filter-out check,$(MAKECMDGOALS)))"; \
	else \
		ACTION="$(or $(ACTION),$(action),$(filter-out check,$(MAKECMDGOALS)))"; \
		if [ -n "$$ACTION" ]; then \
			cargo clippy --manifest-path "actions/$$ACTION/Cargo.toml" -- -D warnings && \
			cargo fmt --manifest-path "actions/$$ACTION/Cargo.toml" -- --check; \
		else \
			for dir in actions/*; do \
				if [ -f "$$dir/Cargo.toml" ]; then \
					cargo clippy --manifest-path "$$dir/Cargo.toml" -- -D warnings && \
					cargo fmt --manifest-path "$$dir/Cargo.toml" -- --check; \
				fi; \
			done; \
		fi; \
	fi

doctor: ## Run Keel doctor diagnostics
	@keel doctor .

validate: ## Run Keel spec validation
	@keel validate .

# Ignore positional arguments passed as targets for positional commands like `make package opencode-usage`
%:
	@:
