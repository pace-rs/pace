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

# The latency guard: fails if pace answers slower than 50 ms
bench:
    cargo bench --bench latency

# Test coverage as lcov.info, as in CI
cov:
    cargo llvm-cov nextest --workspace --all-features --lcov --output-path lcov.info

# Advisories, licenses, duplicate crates and sources
deny:
    cargo deny check

# MSRV and feature powerset checks
hack:
    cargo hack check --rust-version --workspace --all-targets
    cargo hack check --feature-powerset --workspace --all-targets
