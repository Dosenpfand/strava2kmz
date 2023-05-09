use clap::Parser;
use flate2::read::GzDecoder;
use gpx_kml_convert::convert;
use serde::{Deserialize, Deserializer};
use std::{
    fs::{self, File},
    io::{stdout, Read, Write, copy},
    result,
};
use zip::{write::FileOptions, ZipArchive};

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
    medias: Vec<String>,
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
    let kmz_file_name = format!("{}.kmz", record.activity_id);
    let kmz_path = std::path::Path::new(&kmz_file_name);
    let kmz_file = std::fs::File::create(kmz_path).unwrap();
    let mut kmz_writer = zip::ZipWriter::new(kmz_file);
    let zip_options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    {
        let mut track_file: zip::read::ZipFile = zip_file.by_name(&record.filename).unwrap();
        kmz_writer.start_file("doc.kml", zip_options).unwrap();

        if record.filename.ends_with(".gz") {
            let mut gz_decoder = GzDecoder::new(track_file);
            convert(&mut gz_decoder, &mut kmz_writer).unwrap();
        } else {
            convert(&mut track_file, &mut kmz_writer).unwrap();
        }
    }

    {
        for media_file_name in &record.medias {
            kmz_writer.start_file(media_file_name, zip_options).unwrap();
            let mut media_file = zip_file.by_name(&media_file_name).unwrap();
            copy(&mut media_file, &mut kmz_writer).unwrap();
        }
    }

    kmz_writer.finish().unwrap();

    println!("{:?}", record);
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
