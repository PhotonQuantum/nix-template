{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, flake-utils, naersk, nixpkgs, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustDefault = pkgs.rust-bin.stable.latest.default;
        rustMinimal = pkgs.rust-bin.stable.latest.minimal;

        naersk' = pkgs.callPackage naersk {
          cargo = rustMinimal;
          rustc = rustMinimal;
        };

        additionalBuildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.darwin.libiconv pkgs.darwin.Security ];
        devBuildInputs = [ rustDefault ];

      in
      {
        # For `nix build` & `nix run`:
        packages = rec {
          nix-template = naersk'.buildPackage {
            src = ./.;
            cargoBuildOptions = x: x ++ [ "-p" "nix-template" ];
            cargoTestOptions = x: x ++ [ "-p" "nix-template" ];
            nativeBuildInputs = additionalBuildInputs;
          };
          default = nix-template;
        };

        apps = rec {
          nix-template = flake-utils.lib.mkApp { drv = self.packages.${system}.nix-template; };
          default = nix-template;
        };

        # For `nix develop`:
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = devBuildInputs ++ additionalBuildInputs;
        };
      }
    );
}
