use clap::Parser;
use std::{fs, path::PathBuf};

/// Converts a neorg file to pandoc json
#[derive(Parser, Debug)]
struct Args {
    /// Path of the neorg file to process
    file: PathBuf,
}

fn main() {
    let args = Args::parse();
    let mut builder = env_logger::Builder::new();
    builder.filter_level(log::LevelFilter::Info);
    builder.parse_default_env();
    builder.init();

    let file_contents = fs::read_to_string(args.file).expect("Failed to open neorg file");

    let frontend = pandoc_norg_converter::Frontend::new(&file_contents);
    let document = frontend.convert();

    let stdout = std::io::stdout().lock();
    serde_json::to_writer(stdout, &document).expect("Failed to output to stdout");
}
