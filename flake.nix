{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, flake-utils, crane, nixpkgs, rust-overlay, ... }:
    let
      mkCrate = { crane', ... }:
        let
          crateName = crane'.crateNameFromCargoToml { src = ./nix-template; };
        in
        crane'.buildPackage {
          src = crane'.cleanCargoSource ./.;
          pname = crateName.pname;
          version = crateName.version;
        };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustMinimal = pkgs.rust-bin.stable.latest.minimal;
        crane' = (crane.mkLib pkgs).overrideToolchain rustMinimal;
        crate = pkgs.callPackage mkCrate { inherit crane'; };
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
