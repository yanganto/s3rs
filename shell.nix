let
  pkgs = import <nixpkgs> { };
  inherit (pkgs) stdenv;
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustup
    pkg-config
    openssl
  ];
}
