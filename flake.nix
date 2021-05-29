{
  description = "A s3 client written in Rust";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
  let
    pkgs = nixpkgs.legacyPackages.x86_64-linux;
  in {
    packages.x86_64-linux.s3rs = pkgs.rustPlatform.buildRustPackage rec {
        name = "s3rs";
        src = self;
        cargoSha256 = "sha256-tpAbSX6e5nfxn5mwgngZX8I3cfkZFfbx+2Y5/Z1m0g4=";
        nativeBuildInputs = with pkgs; [ python3 perl ];
        buildInputs = with pkgs; [ openssl ];
    };
    defaultPackage.x86_64-linux = self.packages.x86_64-linux.s3rs;
  };
}
