{
  description = "A s3 client written in Rust";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.dependency-refresh.url = "github:yanganto/dependency-refresh";

  outputs = { self, rust-overlay, nixpkgs, flake-utils, dependency-refresh }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.nightly."2024-01-23".default;
        dr = dependency-refresh.defaultPackage.${system};
        updateDependencyScript = pkgs.writeShellScriptBin "update-dependency" ''
          dr ./Cargo.toml
          if [ -f "Cargo.toml.old" ]; then
            rm Cargo.toml.old
            exit 1
          fi
        '';
        publishScript = pkgs.writeShellScriptBin "crate-publish" ''
          cargo login $1
          cargo publish
        '';
      in
      with pkgs;
      rec {
        packages.${system}.s3rs = pkgs.rustPlatform.buildRustPackage {
          name = "s3rs";
          src = self;
          cargoSha256 = "sha256-fbrCbpUMzBxqEBn2ZDLlBQf0d+KA//Rc14rKzKO/fWQ=";
          buildInputs = [ openssl ];
          nativeBuildInputs = [ pkg-config ];
        };
        defaultPackage = packages.${system}.s3rs;
        devShell = mkShell {
          buildInputs = [
            openssl
            pkg-config
            rust
            dr
            publishScript
            updateDependencyScript
          ];
        };
      }
    );
}
