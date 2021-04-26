{ pkgs, lib, openssl, sources, python3, perl }:
let
  mkRustPlatform = pkgs.callPackage ./mk-rust-platform.nix { };
  rustPlatform = mkRustPlatform { };
in
rustPlatform.buildRustPackage rec {
  name = "s3rs";
  src = lib.cleanSource ./.;
  cargoSha256 = "18qwb2fqdigi0gp3am274n3a147lpl6c8kkibwlz2f6k9kpfsnp2";
  nativeBuildInputs = [ python3 perl ];
  buildInputs = [ openssl ];
}
