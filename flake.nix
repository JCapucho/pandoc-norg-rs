{
  inputs = {
    nci.url = "github:yusdacra/nix-cargo-integration";
  };
  outputs = inputs:
    inputs.nci.lib.makeOutputs {
      root = ./.;
      config = common: {
        shell = {
          packages = with common.pkgs; [pandoc rust-analyzer];
        };
        outputs.defaults = {
          app = "pandoc-norg-rs";
          package = "pandoc-norg-rs";
        };
      };
    };
}
