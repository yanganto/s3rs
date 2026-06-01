{
  description = "A s3 client written in Rust";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05-small";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
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
          cargoHash = "sha256-nOqBNX/iKfprynaUueJ9kZpwbXEMSYA9mnUmCR8MEIo=";
          buildInputs = [ openssl ];
          nativeBuildInputs = [ pkg-config ];
        };
        defaultPackage = packages.${system}.s3rs;
        devShell = mkShell {
          buildInputs = [
            openssl
            pkg-config
            rustup
            publishScript
          ];
        };
      }
    );
}
