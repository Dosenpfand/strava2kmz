use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use strava2kmz::{extract_records, Kmz, Record};

/// Convert a strave export archive to a set of kmz files.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    in_file: std::path::PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let in_file = &args.in_file;
    let file =
        fs::File::open(in_file).with_context(|| format!("Could not open {}", in_file.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Could not read {}", in_file.display()))?;

    let records = extract_records(&mut archive)
        .with_context(|| format!("Could not extract all records from {}", in_file.display()))?;

    records
        .into_iter()
        .map(|x: Record| Kmz::convert(&format!("{}.kmz", &x.activity_id()), &mut archive, &x))
        .for_each(drop);
    Ok(())
}
