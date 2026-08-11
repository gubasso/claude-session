# Justfile — task runner for claude-session.
#
# Every recipe runs inside the pinned Nix devShell (flake.nix, devShells.default)
# so local runs, hooks, and CI all use the same toolchain. Override the prefix to
# run against whatever is already on PATH:
#
#     just dev="" test          # skip the devShell
#
# Requires: nix (flakes enabled). The devShell provides the rust toolchain from
# rust-toolchain.toml plus cargo-nextest, cargo-deny, cargo-audit, taplo, just,
# pre-commit, and the nix quality tools.

dev := "nix develop --command"

# List available recipes.
default:
	@just --list

# --- Core gates -------------------------------------------------------------

# Run static analysis: clippy (all targets/features, warnings are errors).
lint:
	{{dev}} cargo clippy --all-targets --all-features -- -D warnings

# Run the test suite (unit + integration) via cargo-nextest.
test:
	{{dev}} cargo nextest run --all-features --no-tests=pass

# Build the debug binary.
build:
	{{dev}} cargo build --all-features

# Build the optimized release binary.
build-release:
	{{dev}} cargo build --release --all-features

# Format Rust sources and TOML.
fmt:
	{{dev}} cargo fmt --all
	{{dev}} taplo format

# Verify formatting without writing (CI-friendly).
fmt-check:
	{{dev}} cargo fmt --all -- --check
	{{dev}} taplo format --check

# Format, lint, then test.
check: fmt lint test

# --- Test lanes (mirror the pre-commit / pre-push / CI tiers) ---------------

# Unit tests only — the pre-commit lane (.config/nextest.toml).
test-unit:
	{{dev}} cargo nextest run --profile pre-commit --all-features --no-tests=pass

# Integration tests only — the pre-push lane (.config/nextest.toml).
test-integration:
	{{dev}} cargo nextest run --profile pre-push --all-features --no-tests=pass

# Full suite with the CI profile (retries, quiet output).
test-ci:
	{{dev}} cargo nextest run --profile ci --all-features --no-tests=pass

# --- Supply chain & docs ----------------------------------------------------

# Audit dependencies for known vulnerabilities.
audit:
	{{dev}} cargo audit

# Run cargo-deny checks (advisories, bans, sources, licenses).
deny:
	{{dev}} cargo deny check advisories bans sources licenses

# Build the crate documentation.
doc:
	{{dev}} cargo doc --all-features --no-deps

# Regenerate the configuration examples and schema from the config types.
#
# Unlike `artifacts`, these ARE checked in: they are documentation a user copies,
# and freshness is a byte comparison the pre-commit hook runs.
gen-config:
	{{dev}} cargo xtask gen-config

# Report a stale generated artifact without writing one.
gen-config-check:
	{{dev}} cargo xtask gen-config --check

# Regenerate the completions and man page, then smoke them.
#
# The output goes under the ignored build directory on purpose: the parser is
# the only source, and a checked-in copy is the drift this recipe exists to
# prevent. A grammar-owning slice runs this as tail work.
artifacts out="target/artifacts":
	mkdir -p {{out}}
	{{dev}} cargo run -p claude-session --all-features -- man > {{out}}/claude-session.1
	for shell in bash elvish fish powershell zsh; do \
		{{dev}} cargo run -p claude-session --all-features -- completion $shell > {{out}}/claude-session.$shell; \
	done
	{{dev}} cargo nextest run --profile pre-push --all-features -E 'binary(cli_artifacts)'

# --- Hooks ------------------------------------------------------------------

# Install the pre-commit, commit-msg, and pre-push hooks.
hooks-install:
	{{dev}} pre-commit install --install-hooks

# Run every pre-commit hook across all files, all stages.
hooks:
	{{dev}} pre-commit run --all-files --hook-stage pre-commit
	{{dev}} pre-commit run --all-files --hook-stage pre-push

# --- Publishing (thin wrappers; see docs/guides/releasing.md) ----------------

# Dry-run the crates.io publish (no upload).
publish-dry:
	./scripts/publish-dry

# Publish the crate to crates.io.
publish:
	./scripts/publish

# Open or refresh the release-plz release PR.
release-pr:
	./scripts/release release-plz-pr

# Publish released versions via release-plz.
release-update:
	./scripts/release release-plz-update

# --- Install (thin wrappers over ./install.sh, ./uninstall.sh) --------------

# Build and install the CLI with cargo (honors PREFIX, INSTALLER_LOCKED).
install:
	./install.sh

# Remove the cargo-installed CLI (honors PREFIX); no-op when not installed.
uninstall:
	./uninstall.sh

# Uninstall then install — the clean-slate refresh.
reinstall: uninstall install

# --- Housekeeping -----------------------------------------------------------

# Run the CLI locally; forward args, e.g. `just run -- --help`.
run *ARGS:
	{{dev}} cargo run -p claude-session --all-features -- {{ARGS}}

# Remove build artifacts.
clean:
	{{dev}} cargo clean
