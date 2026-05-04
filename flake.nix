{
  description = "SPEL admin authority crate";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            rustc
            rustfmt
            pkg-config
            openssl
          ];
        };

        checks.fmt = pkgs.runCommand "spel-admin-authority-fmt" { } ''
          cd ${self}
          find admin-authority admin-authority-sample admin-authority-sample-methods integration-tests \
            -name '*.rs' -print0 | xargs -0 ${pkgs.rustfmt}/bin/rustfmt --edition 2024 --check
          touch $out
        '';
      });
}
