{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    nci.url = "github:yusdacra/nix-cargo-integration";
    nci.inputs.nixpkgs.follows = "nixpkgs";
    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  };
  outputs = inputs @ {
    parts,
    nci,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [nci.flakeModule];
      perSystem = {
        config,
        pkgs,
        ...
      }: let
        crateName = "pandoc-norg-rs";
        crateOutputs = config.nci.outputs.${crateName};
      in {
        nci.projects.${crateName}.relPath = "";
        # configure crates
        nci.crates.${crateName} = {
          export = true;
          overrides = {
            add-tests-inputs.overrideAttrs = old: {
              buildInputs = (old.buildInputs or []) ++ [pkgs.pandoc];
            };
          };
        };
        devShells.default = crateOutputs.devShell.overrideAttrs (old: let
          extraPackages = with pkgs; [cargo-deny pandoc rust-analyzer tree-sitter];
        in {
          packages = (old.packages or []) ++ extraPackages;
        });
        packages.default = crateOutputs.packages.release;
      };
    };
}
