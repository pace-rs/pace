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

# Run the test suite with coverage for the given package
coverage *ARGS: 
	cargo tarpaulin --output-dir coverage/ -p {{ARGS}} -o Lcov