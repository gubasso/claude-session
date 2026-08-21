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
        # Reads channel + components + targets straight from rust-toolchain.toml.
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        # `nix fmt` uses the RFC 166 formatter (also on PATH for the pre-commit hook).
        formatter = pkgs.nixfmt;

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
            # The wrapper itself is deliberately absent, and this flake builds
            # no package that would install it either: `just install` is the one
            # thing that puts `claude-session` on PATH
            # (ADR-0116). Entering this shell — or having direnv enter it —
            # therefore shadows nothing and one binary answers everywhere. Use
            # `just run` for the working tree, which is unambiguous about what
            # it builds.
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
