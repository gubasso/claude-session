{
  description = "claude-session rust CLI dev shell (toolchain from rust-toolchain.toml)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    # The release-kit CLI, pinned at a release tag. `rk devshell sync` moves the
    # pin and the lock together, so the project supplies its own `rk` instead of
    # depending on a host install.
    release-kit = {
      url = "github:gubasso/release-kit/v0.3.3";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      release-kit,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        # Reads channel + components + targets straight from rust-toolchain.toml.
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        # `nix fmt` uses the RFC 166 formatter (also on PATH for the pre-commit hook).
        formatter = pkgs.nixfmt;

        # The wrapper, built from the release-kit-owned expression. The gated
        # pipeline's `flake` job is what consumes it, so the build is proven on
        # every request (ADR-0119). `just install` remains how the binary
        # reaches a developer's PATH (ADR-0116).
        packages.default = pkgs.callPackage ./nix/package.nix { };

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
            # Spell check. Upstream ships it as a `language: python` hook whose
            # wheel carries a generic-glibc binary, which cannot execute here
            # (ADR-0040), so it runs as language:system off PATH.
            pkgs.typos
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
            # The release-kit CLI from this project's own pinned input, so the
            # release convention is read from the flake rather than a host
            # install.
            release-kit.packages.${system}.default
            # The wrapper itself is deliberately absent: `just install` is the
            # one thing that puts `claude-session` on PATH (ADR-0116).
            # Entering this shell — or having direnv enter it — therefore
            # shadows nothing and one binary answers everywhere. Use `just run`
            # for the working tree, which is unambiguous about what it builds.
            # `packages.default` above builds the wrapper for the pipeline's
            # proof (ADR-0119) and installs nothing here.
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
