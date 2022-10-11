{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, flake-utils, nixpkgs, ... }:
    let
      mkCrate = { rustPlatform, ... }:
        let
          cargoToml = builtins.fromTOML (builtins.readFile ./nix-template/Cargo.toml);
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
        in
          rustPlatform.buildRustPackage {
            inherit pname version;
            src = self;
            cargoLock.lockFile = self + "/Cargo.lock";
          };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        crate = pkgs.callPackage mkCrate { };
      in
      {
        # For `nix build` & `nix run`:
        packages = rec {
          nix-template = crate;
          default = nix-template;
        };

        apps = rec {
          nix-template = flake-utils.lib.mkApp { drv = crate; };
          default = nix-template;
          repl = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "repl" ''
              confnix=$(mktemp)
              echo "builtins.getFlake (toString $(git rev-parse --show-toplevel))" >$confnix
              trap "rm $confnix" EXIT
              nix repl $confnix
            '';
          };
        };
      }
    );
}
