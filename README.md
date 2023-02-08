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

# Supported syntax

- Attached modifiers

  - ✅ **Bold**

  - ✅ <u>Underline</u>

  - ✅ ~~Strike-trough~~

  - ⬜ Spoiler

  - ✅ <sup>Superscript</sup>

  - ✅ <sub>Subscript</sub>

  - ✅ `Inline code`

  - ⬜ Null modifier

  - ✅ $\text{Inline math}$

  - ⬜ Variable

  - ✅ Free-form attached modifiers

  - ⬜ Link modifier

  - ⬜ Attached modifier extensions

  - ⬜ Inline comment

- Detached modifiers

  - ✅ Headings

  - ✅ Unordered lists

  - ✅ Ordered lists

  - ✅ Quotes

  - ⬜ Attributes

  - ⬜ Definitions

  - ⬜ Footnotes

  - ⬜ Table cells (The old `@table` syntax is implemented)

  - ⬜ Delimiting modifiers

  - ⬜ Horizontal rule

  - ⬜ Detached modifier extensions

    - ✅ TODO status extension

  - Detached modifier suffix

    - ⬜ Slide

    - ⬜ Indent segment

  - Tags

    - ⬜ Macro tags

    - ✅ Comment ranged tag

    - ✅ Example tag

    - ⬜ Details tag

    - ⬜ Group tag

    - ✅ Code block

    - ⬜ Carryover tags

    - ⬜ Infirm tag

    - ⬜ Image tag

    - ✅ Embed tag

    - ✅ math tag

- Linkables

  - Link location

    - ⬜ File Location

    - ⬜ Line number

    - ✅ Url

    - ⬜ Detached Modifier

    - ⬜ Magic Char

    - ✅ File linkable

    - ⬜ Timestamps

    - ⬜ Wiki links

    - ⬜ Scoping

  - ✅ Link Description

  - ⬜ Anchors

  - ⬜ Inline Linkables

- ✅ Object continuation

# License

The `pandoc-norg-rs` program is licensed under the GNU GPL 3.0 (a copy
of the license can be found in [LICENSE](LICENSE) or at
<https://www.gnu.org/licenses/gpl-3.0.en.html>).

The `pandoc-norg-converter` library is licensed under the GNU LGPL 3.0
(a copy of the license can be found in
[pandoc-norg-converter/LICENSE](pandoc-norg-converter/LICENSE) or at
<https://www.gnu.org/licenses/lgpl-3.0.en.html>).
