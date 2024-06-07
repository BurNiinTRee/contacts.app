{
  inputs = {
    devenv.url = "github:cachix/devenv";
    devenv-root = {
      url = "file+file:///dev/null";
      flake = false;
    };
    fenix.url = "github:nix-community/fenix";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs @ {
    flake-parts,
    fenix,
    devenv,
    devenv-root,
    nixpkgs,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} ({...}: {
      systems = ["x86_64-linux"];

      imports = [devenv.flakeModule];

      perSystem = {
        devenv.shells.default = {
          lib,
          pkgs,
          config,
          ...
        }: {
          devenv.root = let
            devenvRootFileContent = builtins.readFile devenv-root.outPath;
          in
            pkgs.lib.mkIf (devenvRootFileContent != "") devenvRootFileContent;

          # https://github.com/cachix/devenv/issues/528
          containers = lib.mkForce {};
          languages.rust = {
            enable = true;
            mold.enable = true;
            channel = "nightly";
            # rustflags = builtins.toString [
            #   "--cfg"
            #   "tokio_unstable"
            # ];
            # targets = ["x86_64-unknown-linux-gnu"];
            # components = [
            #   "rustc"
            #   "cargo"
            #   "clippy"
            #   "rustfmt"
            #   "rust-analyzer"
            #   "miri"
            #   "rust-src"
            #   "rust-std"
            # ];
          };
          packages = [
            pkgs.cargo-watch
            pkgs.dart-sass
            pkgs.sqlx-cli
            # pkgs.tokio-console
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
          process-managers.process-compose.settings.theme = "Light Modern";
          processes.cargo-watch.exec = "cargo watch --clear -x 'sqlx database setup' -x 'run'";
        };
      };
    });
}
