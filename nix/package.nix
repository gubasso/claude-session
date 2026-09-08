# Seeded by release-kit: a starting point this project owns and tunes;
# release-kit reports drift here and never rewrites it.
#
# Supported shape: one crate with a [package] table — an implicit
# src/main.rs binary or an explicit [[bin]] entry — building with the
# committed Cargo.lock. A workspace root fails by name below rather than
# throwing on a missing attribute: point the importTOML call at the member
# crate's Cargo.toml and set mainProgram yourself.
{ lib, rustPlatform }:

let
  cargoToml = lib.importTOML ../Cargo.toml;
  package =
    cargoToml.package
      or (throw "nix/package.nix: Cargo.toml has no [package] table; this seed does not support a workspace root");
in
rustPlatform.buildRustPackage {
  pname = package.name;
  inherit (package) version;

  src = lib.cleanSource ../.;
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    # The first [[bin]] name where one is declared, else the package
    # name — the implicit src/main.rs binary. nix run resolves the
    # binary through this attribute.
    mainProgram = if cargoToml ? bin then (lib.head cargoToml.bin).name else package.name;
    # Cargo.toml declares `MIT OR Apache-2.0`; a list is how nixpkgs
    # spells a disjunction. Nix cannot derive it from the SPDX string in
    # pure evaluation, so the two are kept in step by hand.
    license = [
      lib.licenses.mit
      lib.licenses.asl20
    ];
  }
  // lib.optionalAttrs (package ? description) { inherit (package) description; }
  // lib.optionalAttrs (package ? homepage) { inherit (package) homepage; };
}
