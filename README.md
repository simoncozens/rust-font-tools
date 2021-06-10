# fonttools-rs &emsp; [![Build Status]][actions] [![Latest Version]][crates.io] [![Docs badge]][docs.rs]

[Build Status]: https://img.shields.io/github/workflow/status/simoncozens/fonttools-rs/build/main
[actions]: https://github.com/simoncozens/fonttools-rs/actions?query=branch%3Amain
[Latest Version]: https://img.shields.io/crates/v/fonttools.svg
[crates.io]: https://crates.io/crates/fonttools
[Docs badge]: https://img.shields.io/badge/docs.rs-rustdoc-green
[docs.rs]: https://docs.rs/fonttools/

This is an attempt to write an Rust library to read, manipulate and
write TTF/OTF files. It is in the early stages of
development. Contributions are welcome. 

# Example usage

```rust
use fonttools::font::{self, Font, Table};
use fonttools::name::{name, NameRecord, NameRecordID};

// Load a font (tables are lazy-loaded)
let fontfile = File::open("Test.otf").unwrap();
use std::fs::File;
let mut myfont = font::load(fontfile).expect("Could not load font");

// Access an existing table
if let Table::Name(name_table) = myfont.get_table(b"name")
        .expect("Error reading name table")
        .expect("There was no name table") {
        // Manipulate the table (table-specific)
        name_table.records.push(NameRecord::windows_unicode(
            NameRecordID::LicenseURL,
            "http://opensource.org/licenses/OFL-1.1"
        ));
}
let mut outfile = File::create("Test-with-OFL.otf").expect("Could not create file");
myfont.save(&mut outfile);
```

See the [https://docs.rs/fonttools](documentation) for more details, and the [https://github.com/simoncozens/fonttools-rs/tree/main/crates/fonttools-cli/src/bin](fonttools-cli utilities) (installable via `cargo install fonttools_cli`) for code examples.

## License

[Apache-2](http://www.apache.org/licenses/LICENSE-2.0)

