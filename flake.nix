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
          ...
        }: {
          # https://github.com/cachix/devenv/issues/528
          containers = lib.mkForce {};
          languages.rust.enable = true;
          packages = [
            pkgs.clippy
            pkgs.cargo-watch
            pkgs.sea-orm-cli
            pkgs.mold
          ];
          env = {
            DATABASE_URL = "sqlite:data.db?mode=rwc";
            CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = ["-Clink-arg=-fuse-ld=mold" "-Clinker=clang"];
          };
        };
      };
    });
}
