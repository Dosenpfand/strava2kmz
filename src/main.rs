use clap::Parser;
use gpx_kml_convert::convert;
use serde::{Deserialize, Deserializer};
use std::{
    fs::{self, File},
    io::{stdout, Read},
    result,
};
use zip::{write::FileOptions, ZipArchive};
use flate2::read::GzDecoder;

/// Convert a strave export archive to a set of kmz files.
#[derive(Parser)]
struct Cli {
    /// The path to the file to read
    in_file: std::path::PathBuf,
}

#[derive(Debug, Deserialize)]
struct Record {
    // TODO: string slices?
    #[serde(rename(deserialize = "Activity ID"))]
    activity_id: String,
    #[serde(rename(deserialize = "Filename"))]
    filename: String,
    #[serde(
        rename(deserialize = "Media"),
        deserialize_with = "Record::deserialize_media"
    )]
    media: Vec<String>,
}

impl Record {
    fn deserialize_media<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
    where
        D: Deserializer<'de>,
        &'de str: Deserialize<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Ok(s.split('|')
            .filter(|&x| !x.is_empty())
            .map(|s: &str| s.to_string())
            .collect())
    }
}

fn extract_records(zip_file: &mut zip::ZipArchive<File>) -> Vec<Record> {
    let activities_file = zip_file.by_name("activities.csv").unwrap();
    let mut rdr = csv::Reader::from_reader(activities_file);
    rdr.deserialize().map(|x| x.unwrap()).collect()
}

fn write_kmz(zip_file: &mut zip::ZipArchive<File>, record: &Record) {
    let mut record_file = zip_file.by_name(&record.filename).unwrap();
    
    let out_file_name = format!("{}.kmz", record.activity_id);
    let path = std::path::Path::new(&out_file_name);
    let file = std::fs::File::create(path).unwrap();
    let mut zip_writer = zip::ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);
    zip_writer.start_file("doc.kml", options).unwrap();

    if record.filename.ends_with(".gz") {
        let mut decoder = GzDecoder::new(record_file);
        convert(&mut decoder, &mut zip_writer).unwrap();
    }
    else {
        convert(&mut record_file, &mut zip_writer).unwrap();
    }

    println!("{:?}", record);

    zip_writer.finish().unwrap();
}

fn main() {
    let args = Cli::parse();
    let file = fs::File::open(args.in_file).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    let records = extract_records(&mut archive);
    records
        .into_iter()
        .map(|x| write_kmz(&mut archive, &x))
        .for_each(drop);
}
