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
        }: let
          sqlite-icu-extension = pkgs.stdenv.mkDerivation (attrs: {
            name = "sqlite3-icu";

            src = pkgs.fetchurl {
              url = "https://sqlite.org/src/raw/c074519b46baa484bb5396c7e01e051034da8884bad1a1cb7f09bbe6be3f0282?at=icu.c";
              hash = "sha256-1jGW8jT/UaGgk/9yeD3kU2Y8hDwYvqtuzlaTPBPO5bo=";
            };
            dontUnpack = true;
            nativeBuildInputs = [
              pkgs.pkg-config
            ];
            buildInputs = [
              pkgs.icu
              pkgs.sqlite
            ];
            buildPhase = ''
              $CC -fPIC -shared $src $(pkg-config --libs --cflags icu-uc icu-io) -o libSqliteIcu.so
            '';
            installPhase = ''
              install -D -t $out/lib/ libSqliteIcu.so
            '';
          });
        in {
          # https://github.com/cachix/devenv/issues/528
          containers = lib.mkForce {};
          languages.rust = {
            enable = true;
            # channel = "nightly";
            # components = [
            #   "cargo"
            #   "clippy"
            #   "llvm-tools-preview"
            #   "rust-analyzer"
            #   "rust-src"
            #   "rustc"
            #   "rustfmt"
            # ];
          };
          packages = [
            pkgs.cargo-watch
            pkgs.dart-sass
            pkgs.mold
            pkgs.sqlx-cli
            pkgs.tokio-console
            pkgs.vscode-langservers-extracted
          ];
          env = {
            DATABASE_URL = "postgresql:///contacts";
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = [
              "-Clink-arg=-fuse-ld=mold"
              "-Clinker=clang"
              "--cfg"
              "tokio_unstable"
            ];
            RUST_LOG = "info";
            SQLITE_ICU_EXTENSION = sqlite-icu-extension + /lib/libSqliteIcu.so;
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
