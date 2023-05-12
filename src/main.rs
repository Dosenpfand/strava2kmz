use anyhow::{Context, Result};
use clap::Parser;
use indicatif::ProgressIterator;
use std::{
    fs::{self, File},
    path::PathBuf,
};
use strava2kmz::{Activity, KmzConverter};

/// Convert a strave export archive to a set of kmz files.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    in_file: PathBuf,
    /// The directory where the output is written
    out_dir: Option<PathBuf>,
    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();
    let in_file = &args.in_file;
    let out_dir = &args.out_dir.unwrap_or_else(PathBuf::new);

    let file =
        fs::File::open(in_file).with_context(|| format!("Could not open {}", in_file.display()))?;

    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Could not read {}", in_file.display()))?;

    let mut activities_file = archive
        .by_name("activities.csv")
        .with_context(|| format!("Could not find activities.csv in {}", in_file.display()))?;

    let records = Activity::extract_records(&mut activities_file)
        .with_context(|| format!("Could not extract all records from {}", in_file.display()))?;
    drop(activities_file);

    records
        .into_iter()
        .progress()
        .try_for_each(|x: Activity| {
            let mut out_path = out_dir.clone();
            out_path.push(x.activity_id());
            out_path.set_extension("kmz");
            KmzConverter::<File>::convert(&out_path.to_string_lossy(), &mut archive, &x)
        })
        .context("Could not convert to kmz")
}
