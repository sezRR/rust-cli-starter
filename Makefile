SHELL := /bin/bash
.DEFAULT_GOAL := help

# Everything below follows the package declared in Cargo.toml, so renaming the
# package is enough to rename the binary, its release assets, and its env vars.
CARGO_METADATA = cargo metadata --locked --no-deps --format-version 1 2>/dev/null
PACKAGE_NAME = $(shell $(CARGO_METADATA) | jq -r '.packages[0].name')
PACKAGE_VERSION = $(shell $(CARGO_METADATA) | jq -r '.packages[0].version')
ENV_PREFIX = $(shell printf '%s' '$(PACKAGE_NAME)' | tr 'a-z-' 'A-Z_')
VERSION ?= $(PACKAGE_VERSION)
TAG = v$(VERSION)

.PHONY: help run sync-lock fmt check test lint build verify install install-version uninstall release

help:
	@printf '%s\n' \
		'make run                       Run the example CLI' \
		'make sync-lock                 Synchronize Cargo.lock after a version change' \
		'make check                     Check the locked Rust build' \
		'make test                      Run Rust tests' \
		'make lint                      Check Rust, shell, and workflow files' \
		'make verify                    Run all release checks' \
		'make install                   Install the latest release' \
		'make install-version VERSION=x Install a specific release' \
		'make uninstall                 Remove the installed binary and its completions' \
		'make release                   Tag and push the Cargo package version'

run:
	cargo run -- hello

sync-lock:
	cargo check

fmt:
	cargo fmt --check

check:
	cargo check --locked

test:
	cargo test --locked

lint: fmt
	cargo clippy --all-targets --locked -- -D warnings
	shellcheck scripts/*.sh
	actionlint .github/workflows/release.yml

build:
	cargo build --release --locked

verify: check test lint build

install:
	bash scripts/install.sh

install-version:
	@test -n "$(VERSION)" || { printf 'VERSION is required.\n' >&2; exit 1; }
	$(ENV_PREFIX)_VERSION="$(VERSION)" bash scripts/install.sh

uninstall:
	bash scripts/uninstall.sh

release: verify
	@test -n "$(VERSION)" || { printf 'Cargo package version could not be determined.\n' >&2; exit 1; }
	@test "$(VERSION)" = "$(PACKAGE_VERSION)" || { printf 'VERSION %s does not match Cargo package version %s.\n' "$(VERSION)" "$(PACKAGE_VERSION)" >&2; exit 1; }
	@test -z "$$(git status --porcelain)" || { printf 'Commit or stash worktree changes before releasing.\n' >&2; exit 1; }
	git tag "$(TAG)"
	git push origin "$(TAG)"
