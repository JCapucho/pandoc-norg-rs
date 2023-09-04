use clap::Parser;
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Converts a neorg file to pandoc json
#[derive(Parser, Debug)]
struct Args {
    /// Path of the neorg file to process
    file: Option<PathBuf>,
}

fn read_from_stdin() -> String {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .expect("Failed to run from stdin");
    String::from_utf8(input).expect("Non UTF8 input on stdin")
}

fn main() {
    let args = Args::parse();
    let mut builder = env_logger::Builder::new();
    builder.filter_level(log::LevelFilter::Info);
    builder.parse_default_env();
    builder.init();

    let file_contents = match args.file {
        None => read_from_stdin(),
        Some(p) if p == Path::new("-") => read_from_stdin(),
        Some(path) => fs::read_to_string(path).expect("Failed to open neorg file"),
    };

    let mut frontend = pandoc_norg_converter::Frontend::default();
    let document = frontend.convert(&file_contents);

    let stdout = std::io::stdout().lock();
    serde_json::to_writer(stdout, &document).expect("Failed to output to stdout");
}
