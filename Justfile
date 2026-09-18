set shell := ["bash", "-eu", "-c"]

# Default recipe: list available commands
default:
    @just --list

# Package a specific action by name (e.g. `just package opencode-usage`)
package action:
    @./tools/package.sh "{{action}}"

# Package all actions in the repository
package-all:
    #!/usr/bin/env bash
    set -eu
    for dir in actions/*; do
        if [ -d "$dir" ]; then
            ./tools/package.sh "$(basename "$dir")"
        fi
    done

# Run tests for all actions, or a specific action (e.g. `just test opencode-usage`)
test action="":
    #!/usr/bin/env bash
    set -eu
    if [ -n "{{action}}" ]; then
        if [ ! -d "actions/{{action}}" ]; then
            echo "Error: Action '{{action}}' not found."
            exit 1
        fi
        cargo test --manifest-path "actions/{{action}}/Cargo.toml"
    else
        for dir in actions/*; do
            if [ -f "$dir/Cargo.toml" ]; then
                echo "==> Testing $(basename "$dir")..."
                cargo test --manifest-path "$dir/Cargo.toml"
            fi
        done
    fi

# Run cargo clippy and fmt checks for all actions, or a specific action
check action="":
    #!/usr/bin/env bash
    set -eu
    if [ -n "{{action}}" ]; then
        if [ ! -d "actions/{{action}}" ]; then
            echo "Error: Action '{{action}}' not found."
            exit 1
        fi
        cargo clippy --manifest-path "actions/{{action}}/Cargo.toml" -- -D warnings
        cargo fmt --manifest-path "actions/{{action}}/Cargo.toml" -- --check
    else
        for dir in actions/*; do
            if [ -f "$dir/Cargo.toml" ]; then
                echo "==> Checking $(basename "$dir")..."
                cargo clippy --manifest-path "$dir/Cargo.toml" -- -D warnings
                cargo fmt --manifest-path "$dir/Cargo.toml" -- --check
            fi
        done
    fi

# Run Keel doctor preflight diagnostics
doctor:
    keel doctor .

# Run Keel spec validation
validate:
    keel validate .
