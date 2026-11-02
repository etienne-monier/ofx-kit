#![forbid(unsafe_code)]

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "ofx",
    version,
    about = "Inspect and work with Open Financial Exchange files"
)]
struct Cli {}

fn main() {
    let _ = Cli::parse();
}
