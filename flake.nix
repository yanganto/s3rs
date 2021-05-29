{
  description = "A s3 client written in Rust";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
  let
    pkgs = nixpkgs.legacyPackages.x86_64-linux;
  in {
    packages.x86_64-linux.s3rs-hello = pkgs.stdenv.mkDerivation {
        name = "s3rs-hello";
        src = self;
        installPhase = ''
          mkdir -p $out/bin;
          install -m755 example.sh $out/bin/s3rs-hello;
        '';
    };
    defaultPackage.x86_64-linux = self.packages.x86_64-linux.s3rs-hello;
  };
}
