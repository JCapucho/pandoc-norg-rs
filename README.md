# `pandoc-norg-rs`

A pandoc json filter for parsing
[neorg](https://github.com/nvim-neorg/neorg) documents into the pandoc
ast, written in Rust.

Also take a look at
[Simre1/neorg-haskell-parser](https://github.com/Simre1/neorg-haskell-parser).

## Usage

To use simply run `pandoc-norg-rs` with the neorg file to be converted
and pipe the output to pandoc.

    pandoc-norg-rs <file> | pandoc -f json

# Library

The functionality is also provided has a rust library, the library can
be found in the `pandoc-norg-converter` directory.

# Using with nix

A flake is also provided to be used with nix, the flake provides a
devShell and an app/package to easily add the `pandoc-norg-rs` binary to
a derivation or to be able to run it directly without installing.

    nix run github:JCapucho/pandoc-norg-rs <file> | pandoc -f json

# License

The `pandoc-norg-rs` program is licensed under the GNU GPL 3.0 (a copy
of the license can be found in [LICENSE](LICENSE) or at
<https://www.gnu.org/licenses/gpl-3.0.en.html>).

The `pandoc-norg-converter` library is licensed under the GNU LGPL 3.0
(a copy of the license can be found in
[pandoc-norg-converter/LICENSE](pandoc-norg-converter/LICENSE) or at
<https://www.gnu.org/licenses/lgpl-3.0.en.html>).
