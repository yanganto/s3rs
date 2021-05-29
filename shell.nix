let
  mozillaOverlay =
    import (builtins.fetchGit {
      url = "https://github.com/mozilla/nixpkgs-mozilla.git";
      rev = "57c8084c7ef41366993909c20491e359bbb90f54";
    });
  nixpkgs = import <nixpkgs> { overlays = [ mozillaOverlay ]; };
  rust-stable = with nixpkgs; ((rustChannelOf { date = "2021-05-06"; channel = "stable"; }).rust.override {
  });
in
with nixpkgs; pkgs.mkShell {
  buildInputs = [
    pkg-config
    rust-stable
    openssl
  ];
}
