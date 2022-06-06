# Rust Font Tools

This is a collection of crates for working on (and compiling) OpenType fonts in Rust. It also contains the Fonticulus font compiler. If you're just interested in compiling fonts quickly, see the section [Fonticulus Installation](#Fonticulus-Installation) below.

## Components

* `babelfont-rs`: A library for loading and representing *source* font files (Glyphs 3, UFO, Designspace, Fontlab VI VFJ) into a common set of objects.
* `designspace`: A library for reading `.designspace` files.
* `dschecker`: A tool for checking Designspace formatting and compatibility issues.
* `fonticulus`: A fonticulusly fast font compiler.
* `fonttools-cli`: Various command line utilities using the `fonttools-rs` library.
* `fonttools-rs`: A high-level library for parsing and creating OpenType and TrueType *binary* fonts.
* `openstep-plist`: A library for reading OpenStep-style plist fonts (used by `babelfont-rs` to handle Glyphs files).
* `otmath`: A library for various common OpenType-related mathematical operations, rounding, interpolation and so on.
* `otspec`: A low-level library for parsing and creating OpenType and TrueType binary fonts.
* `otspec-macros`: A set of proc_macros for serializing and deserializing OpenType binary data into Rust structures.
* `triangulate`: A work-in-progress UFO interpolator.

## Fonticulus Installation

First:
[Install Rust](https://doc.rust-lang.org/book/ch01-01-installation.html)

Then:
```
cargo install --git https://github.com/simoncozens/rust-font-tools fonticulus
fonticulus --help
```

This will install the latest cutting-edge version directly from the repo, which is probably what you want to be using at this point while Fonticulus is in alpha stage.

## `dschecker`

```
cargo install --git https://github.com/simoncozens/rust-font-tools dschecker
dschecker --help
```

`dschecker` quickly checks `.designspace` files for correctness and for interpolation incompatibilities. It has two output modes; by default, it prints a human-readable textual description of any problems it finds. When the `-j` flag is passed on the command line, it prints a report in JSON format, for use when called by other programs.

## `triangulate`

```
% cargo install --git https://github.com/simoncozens/rust-font-tools dschecker
% triangulate Nunito.designspace ital=0 wght=450
Saved on Nunito-ital0-wght450.ufo
```

`triangulate` interpolates a font, creating a UFO file representing the font
at a given point in the design space.
