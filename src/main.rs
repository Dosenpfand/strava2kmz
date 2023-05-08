use clap::Parser;

/// TODO
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    in_file: std::path::PathBuf,
}

fn main() {
    let args = Cli::parse();

    println!("{0}", args.in_file.display());

    println!("Hello, world!");
}
