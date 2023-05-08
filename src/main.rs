use std::{fs, result};

use clap::Parser;

use serde::{Deserialize, Deserializer};

/// TODO
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    in_file: std::path::PathBuf,
}

#[derive(Debug, Deserialize)]
struct InRecord {
    // TODO: not possible to have string slices?
    #[serde(rename(deserialize = "Activity ID"))]
    activity_id: String,
    #[serde(rename(deserialize = "Media"), deserialize_with = "InRecord::deserialize_media")]
    media: Vec<String>,
}

impl InRecord  {
    fn deserialize_media<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
        &'de str: Deserialize<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(s.split("|").map(|s| s.to_string()).collect())
    }
}

fn main() {
    let args = Cli::parse();
    let file = fs::File::open(args.in_file).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();
    let activities_file = archive.by_name("activities.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(activities_file);

    for result in rdr.deserialize() {
        let mut record: InRecord = result.unwrap();

        println!("{:?}", &record);
    }
}
