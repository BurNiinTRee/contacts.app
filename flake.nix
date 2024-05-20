{
  inputs = {
    bntr.url = "github:BurNiinTRee/nix-sources?dir=modules";
    devenv.url = "github:cachix/devenv";
    fenix.url = "github:nix-community/fenix";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs @ {
    flake-parts,
    fenix,
    bntr,
    devenv,
    nixpkgs,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} ({...}: {
      systems = ["x86_64-linux"];

      imports = [bntr.flakeModules.nixpkgs devenv.flakeModule];

      perSystem = {
        nixpkgs.overlays = [fenix.overlays.default];
        devenv.shells.default = {
          lib,
          pkgs,
          config,
          ...
        }: {
          # https://github.com/cachix/devenv/issues/528
          containers = lib.mkForce {};
          languages.rust = {
            enable = true;
            channel = "nightly";
            rustflags = builtins.toString [
              # "-Clink-arg=-fuse-ld=mold"
              # "-Clinker=clang"
              "--cfg"
              "tokio_unstable"
            ];
            mold.enable = true;
            components = [
              "rustc"
              "cargo"
              "clippy"
              "rustfmt"
              "rust-analyzer"
              "miri"
              "rust-src"
              "rust-std"
            ];
          };
          packages = [
            pkgs.cargo-expand
            pkgs.cargo-watch
            pkgs.dart-sass
            pkgs.mold
            pkgs.sqlx-cli
            pkgs.tokio-console
            pkgs.vscode-langservers-extracted
          ];
          env = {
            DATABASE_URL = "postgresql:///contacts";
            RUST_LOG = "info";
          };
          services.postgres = {
            enable = true;
            initialDatabases = [
              {
                name = "contacts";
              }
            ];
          };
          process.implementation = "overmind";
          processes.cargo-watch.exec = "cargo watch --clear -x 'sqlx database setup' -x 'run'";
        };
      };
    });
}
