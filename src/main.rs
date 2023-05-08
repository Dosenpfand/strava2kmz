use std::fs;

use clap::Parser;

use serde::Deserialize;

/// TODO
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    in_file: std::path::PathBuf,
}

#[derive(Debug, Deserialize)]
struct InRecord {
    #[serde(rename(deserialize = "Activity ID"))]
    activity_id: String,
    #[serde(rename(deserialize = "Media"))]
    photos: String,
}

fn main() {
    let args = Cli::parse();
    let file = fs::File::open(args.in_file).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut activities_file = archive.by_name("activities.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(activities_file);

    for result in rdr.deserialize() {
        let record: InRecord = result.unwrap();

        println!("{:?}", record);
    }
}
