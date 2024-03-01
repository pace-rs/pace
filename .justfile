################################################################################
#                                   Justfile                                   #
#                                                                              #
# Set of routines to execute for development work.                             #
#                                                                              #
# To make use of this file install: https://crates.io/crates/just              #
#                                                                              #
################################################################################

# 'Just' Configuration

# Loads .env file for variables to be used in
# in this just file 
# set dotenv-load

default:
    just --choose

# Ignore recipes that are commented out
set ignore-comments := true

# Set shell for Windows OSs:
# If you have PowerShell Core installed and want to use it,
# use `pwsh.exe` instead of `powershell.exe`
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Set shell for non-Windows OSs:
set shell := ["bash", "-uc"]

# Runs the benchmark suite
# bench *ARGS:
# 	cargo +nightly bench {{ARGS}}

# Builds the library.
# build:
# 	cargo build --no-default-features
# 	cargo build --all-features
# 	@cargo build --all-features --example sieve
# 	@cargo build --all-features --example tour

# Checks the library for syntax and HIR errors.
check:
	cargo check --no-default-features
	cargo check --all-features

# Runs all of the recipes necessary for pre-publish.
# checkout: format check lint build doc test package

# Continually runs the development routines.
ci:
	just loop dev

# Removes all build artifacts.
clean:
	cargo clean

# Runs the development routines.
dev: format lint doc test

# Opens the crate documentation.
# @cargo +nightly doc --all-features {{ARGS}}
# @cargo doc --all-features --no-deps --open {{ARGS}}
doc *ARGS:
	@cargo doc --all-features --no-deps {{ARGS}}

# Format all code
fmt:
    @just format
    @dprint fmt

# Runs the formatter on all Rust files.
format:
	@cargo +nightly fmt --all

# Runs the linter.
lint: check
	cargo clippy --no-default-features
	cargo clippy --all-features

# Continually runs some recipe from this file.
loop action:
	watchexec -w src -- "just {{action}}"

# Looks for undefined behavior in the (non-doc) test suite.
miri *ARGS:
	cargo +nightly miri test --all-features -q --lib --tests {{ARGS}}

# Packages the crate in preparation for publishing on crates.io
# package:
# 	cargo package --allow-dirty

# Publishes the crate to crates.io
# publish: checkout
# 	cargo publish

# Runs the test suites.
test: check lint
    cargo nextest run --all-features --workspace

# Runs the test suites.
dtest: check lint
    cargo test --doc --workspace

# Runs the whole test suite with nextest.
ntest:
    cargo nextest run --all-features --workspace

# Runs only the ignored tests with nextest.
nitest:
    cargo nextest run --all-features --workspace -- --ignored

# Runs a test defined by an expression with nextest.
# e.g. `just ntest completions` => test completions 
natest *ARGS:
    cargo nextest run --all-features --workspace -E 'test({{ARGS}})'

# list the inverse dependencies
# as in which feature enables a given crate
inv-ft *ARGS:
	cargo tree -e features -i {{ARGS}}

# prepare for making a PR
pr:
	just fmt lint test

# prepare for making a PR (expensive)
ex-pr:
	just pr
	just check-powerset pace_core
	just test-powerset pace_core

# Run the test suite with coverage for the given package
coverage *ARGS: 
	cargo tarpaulin --output-dir coverage/ -p {{ARGS}} -o Lcov

# Run checks with feature powerset
check-powerset *ARGS:
	cargo hack check --feature-powerset -p {{ARGS}}

# Run checks with feature powerset
test-powerset *ARGS:
	cargo hack test --feature-powerset -p {{ARGS}}

# Update the scoop manifest from the given version to the latest on crates.io
update-scoop-manifest *ARGS:
	sd {{ARGS}} $(xh get https://crates.io/api/v1/crates/pace-rs | jq .crate.max_version) scoop/pace.json

# Run insta tests in review mode
insta:
	cargo insta test --review --workspace

# Create a new tag for the current version on crates.io to make cargo-dist create a new release
tag-release:
	git tag -a pace-rs-v$(xh get https://crates.io/api/v1/crates/pace-rs | jq .crate.max_version)
	git push origin pace-rs-v$(xh get https://crates.io/api/v1/crates/pace-rs | jq .crate.max_version)

# Make the most recent version from crates.io the latest release on GitHub
make-latest:
	gh release edit pace-rs-v$(xh get https://crates.io/api/v1/crates/pace-rs | jq .crate.max_version) --latest
