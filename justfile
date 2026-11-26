# Development tasks for pace. Run `just` to list them.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
    @just --list

# Format Rust, Markdown, TOML and JSON
fmt:
    cargo fmt --all
    dprint fmt

# Clippy with warnings as errors, as in CI
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run all tests
test:
    cargo nextest run --workspace --all-features

# Everything CI checks first: formatting, lints, tests
check:
    cargo fmt --all -- --check
    dprint check
    just lint
    just test
