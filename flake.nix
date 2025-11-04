{
  description = "A s3 client written in Rust";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.dependency-refresh.url = "github:yanganto/dependency-refresh";

  outputs = { self, nixpkgs, flake-utils, dependency-refresh }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
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
          cargoHash = "sha256-c41nZmtTrD5iqh38LZGhtpqK5ni/U2KniMXg/2EfXWM=";
          buildInputs = [ openssl ];
          nativeBuildInputs = [ pkg-config ];
        };
        defaultPackage = packages.${system}.s3rs;
        devShell = mkShell {
          buildInputs = [
            openssl
            pkg-config
            rustup
            dr
            publishScript
            updateDependencyScript
          ];
        };
      }
    );
}
