{
  description = "claude-session rust CLI dev shell (toolchain from rust-toolchain.toml)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        inherit (pkgs) lib;
        # Reads channel + components + targets straight from rust-toolchain.toml.
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        # Build with the pinned toolchain rather than whatever rustc nixpkgs
        # carries, so the package and the devShell agree on the compiler
        # (ADR-0019).
        rustPlatform = pkgs.makeRustPlatform {
          cargo = toolchain;
          rustc = toolchain;
        };
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        # An explicit file set, because `src = ./.` would drag `target/` and the
        # whole docs tree into the store and rebuild the package on any of them.
        # `xtask/` is here only because it is a workspace member cargo must
        # resolve, not because the wrapper needs it at runtime.
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./.cargo
            ./src
            ./xtask
          ];
        };
        # The devshell serves the working tree, never a store snapshot. Putting
        # `packages.default` on PATH pins the binary to whatever the flake
        # evaluated at shell entry, so it silently lags every edit made since,
        # and a manual check then reports behaviour the tree no longer has.
        #
        # Build first, then `exec` the real binary: the shim leaves no process
        # between the caller and the wrapper, so the signals, streams, and
        # status a manual passthrough check observes are the ones a direct
        # invocation produces (ADR-0084). `cargo run` would supervise instead,
        # which is the contract under test.
        devWrapper = pkgs.writeShellApplication {
          name = "claude-session-rs";
          runtimeInputs = [ toolchain ];
          text = ''
            root="$(dirname "$(cargo locate-project --workspace --message-format plain)")"
            cargo build --quiet --manifest-path "$root/Cargo.toml" \
              --package claude-session --all-features
            exec "''${CARGO_TARGET_DIR:-$root/target}/debug/claude-session-rs" "$@"
          '';
        };
      in
      {
        # `nix fmt` uses the RFC 166 formatter (also on PATH for the pre-commit hook).
        formatter = pkgs.nixfmt;

        packages.default = rustPlatform.buildRustPackage {
          pname = "claude-session-rs";
          inherit (cargoToml.package) version;
          inherit src;
          cargoLock.lockFile = ./Cargo.lock;
          # Only the wrapper binary; `xtask` is a development entry point that
          # nothing installs.
          cargoBuildFlags = [
            "--package"
            "claude-session"
          ];
          # The suite reads `docs/` and shells out to `script`/`setsid`, neither
          # of which the build sandbox has, so the gate stays with `just hooks`
          # (docs/reference/testing-and-quality.md).
          doCheck = false;
          meta = {
            inherit (cargoToml.package) description;
            homepage = cargoToml.package.repository;
            license = with lib.licenses; [
              mit
              asl20
            ];
            mainProgram = "claude-session-rs";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.cargo-nextest
            pkgs.cargo-deny
            pkgs.cargo-audit
            pkgs.cargo-machete
            # TOML formatter for the local `taplo format` pre-commit hook.
            pkgs.taplo
            # JSON/JSONC + markdown formatter; the dprint pre-commit hooks run as
            # language:system and require it on PATH.
            pkgs.dprint
            pkgs.just
            pkgs.pre-commit
            # Nix quality tools for the pre-commit `_nix` overlay hooks
            # (nixfmt/statix/deadnix run as language:system off PATH).
            pkgs.nixfmt
            pkgs.statix
            pkgs.deadnix
            # Spell check and commit-message lint. Upstream ships these as
            # `language: python` hooks whose wheel carries a generic-glibc
            # binary, which cannot execute here (ADR-0040), so both run as
            # language:system off PATH.
            pkgs.typos
            pkgs.committed
            # Node for the markdownlint-cli2 pre-commit hook, which is pinned to
            # `language_version: system` because pre-commit's own nodeenv
            # fallback downloads a generic-glibc node (ADR-0040). npm still
            # installs the hook's pure-JS custom rule, which nixpkgs does not
            # package.
            pkgs.nodejs
            # util-linux supplies `script` and `setsid`, which the account
            # login integration tests use to allocate and to detach a
            # controlling terminal. Without it the tests would read those
            # binaries off the host PATH, which self-containment forbids.
            pkgs.util-linux
            # The wrapper itself, so entering the shell (or `direnv allow`) puts
            # `claude-session-rs` on PATH without a separate install step. The
            # shim, not `packages.${system}.default`, so what PATH resolves is
            # the working tree rather than the last evaluation of it.
            devWrapper
          ];
          # native deps for -sys crates, uncomment as needed:
          # buildInputs = [ pkgs.openssl ];
          # nativeBuildInputs = [ pkgs.pkg-config ];
          # The greeting goes to standard error, because `nix develop --command`
          # shares the command's stdout: on stdout this banner is prepended to
          # whatever the command emits, which silently corrupts every redirected
          # artifact (`just artifacts`, `just run -- man > page.1`).
          shellHook = ''echo "claude-session dev shell ready (toolchain from rust-toolchain.toml)" >&2'';
        };
      }
    );
}
