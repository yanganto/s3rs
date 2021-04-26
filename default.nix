let
  sources = import ./nix/sources.nix;
  pkgs = import sources.nixpkgs {
    overlays = [
      (_: _: { inherit sources; })
      (import ./overlay.nix)
    ];
  };
in
with pkgs;
stdenv.mkDerivation {
  name = "s3rs";
  nativeBuildInputs = [ s3rs.nativeBuildInputs ];
  src = pkgs.lib.cleanSource ./.;
  buildInputs = [ s3rs ];
  installPhase = ''
    mkdir -p $out/bin
    cp bin/s3rs $out/bin
  '';
}
