{
  description = "Rule based data annotation";

  inputs = {
    nixpkgs.url = "https://channels.nixos.org/nixos-unstable/nixexprs.tar.zst";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
    pre-commit-hooks.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ self, nixpkgs, ... }:
    let
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs [
          "x86_64-linux"
          "aarch64-linux"
          "x86_64-darwin"
          "aarch64-darwin"
        ] f;
      nixpkgsFor = forEachSystem (
        system:
        import nixpkgs {
          inherit system;
        }
      );
    in
    {
      packages = forEachSystem (
        system:
        let
          pkgs = nixpkgsFor.${system};
          rustArtifacts = pkgs.callPackage ./. { };
        in
        {

          default = self.packages.${system}.web;
          inherit (rustArtifacts) web native;

        }
      );

      checks = forEachSystem (system: {
        pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            nixfmt.enable = true;
            deadnix.enable = true;
            rustfmt = {
              enable = true;
              settings.manifest-path = "Cargo.toml";
            };
          };
        };
      });

      devShells = forEachSystem (system: {
        default =
          let
            pkgs = nixpkgsFor.${system};
            inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
          in
          pkgs.mkShell rec {
            inherit shellHook;
            buildInputs =
              enabledPackages
              ++ (with pkgs; [
                cargo
                rustc
                trunk
                pkg-config
                clippy
                rustc.llvmPackages.lld
                wasm-bindgen-cli
                libglvnd
                libx11
                libxcursor
                libxi
                libxcb
                libxkbcommon
                wayland
                dbus
                postgresql
                pre-commit
              ]);
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
          };
      });

    };

}
