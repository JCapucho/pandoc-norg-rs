{
  inputs = {
    nci.url = "github:yusdacra/nix-cargo-integration";
  };
  outputs = inputs:
    inputs.nci.lib.makeOutputs {
      root = ./.;
      config = common: {
        shell = {
          packages = with common.pkgs; [cargo-deny pandoc rust-analyzer tree-sitter];
        };
        outputs.defaults = {
          app = "pandoc-norg-rs";
          package = "pandoc-norg-rs";
        };
      };
      pkgConfig = common: {
        pandoc-norg-rs.overrides = {
          add-test-inputs = {
            buildInputs = [common.pkgs.pandoc];
          };
        };
      };
    };
}
