{
  description = "Frontier WASM canvas demo environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rustfmt"
            "clippy"
            "rust-analyzer"
          ];
          targets = [
            "wasm32-wasip1"
          ];
        };

        platformLibs =
          pkgs.lib.optionals pkgs.stdenv.isLinux (
            with pkgs; [
              libxkbcommon
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
              vulkan-loader
            ]
          )
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
            with pkgs; [
              libiconv
            ]
          );

        ciScript = pkgs.writeShellApplication {
          name = "frontier-wasm-ci";
          runtimeInputs = [ pkgs.nix pkgs.bash ];
          text = ''
            exec nix develop .#default --command ${pkgs.bash}/bin/bash ${./scripts/ci.sh}
          '';
        };
      in
      {
        packages = {
          ci = ciScript;
          default = ciScript;
        };

        apps = {
          default = flake-utils.lib.mkApp {
            drv = ciScript;
            name = "frontier-wasm-ci";
          };
          ci = flake-utils.lib.mkApp {
            drv = ciScript;
            name = "frontier-wasm-ci";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            cargo-component
            just
            pkg-config
            wasm-tools
            wasmtime
          ];
          buildInputs = platformLibs;

          shellHook = ''
            export RUST_BACKTRACE=1
            export WASMTIME_BACKTRACE_DETAILS=1
            echo "Frontier WASM demo environment"
            echo "Use 'just demo' to launch the desktop demo"
            echo "Use 'just ci' to run the CI checks"
          '';
        };
      }
    );
}
