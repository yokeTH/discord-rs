{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    git-hooks,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        pre-commit-check = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            rustfmt.enable = true;
            clippy.enable = true;
            taplo.enable = true;
          };
        };
      in {
        checks = {
          inherit pre-commit-check;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.taplo
            pkgs.cargo-watch
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            ${pre-commit-check.shellHook}
          '';
        };
      }
    );
}
