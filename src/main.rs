use clap::Parser;
use flate2::read::GzDecoder;
use gpx_kml_convert::convert;
use kml::types::Icon;
use serde::{Deserialize, Deserializer};
use std::{
    fs::{self, File},
    io::copy,
    result,
};
use zip::write::FileOptions;

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

struct Kmz<'a> {
    kmz_writer: zip::ZipWriter<File>,
    zip_file: &'a mut zip::ZipArchive<File>,
}

impl<'a> Kmz<'a> {
    fn new(kmz_file_name: &str, zip_file: &'a mut zip::ZipArchive<File>) -> Kmz<'a> {
        let kmz_path = std::path::Path::new(kmz_file_name);
        let kmz_file = std::fs::File::create(kmz_path).unwrap();
        let kmz_writer = zip::ZipWriter::new(kmz_file);
        Kmz {
            kmz_writer,
            zip_file,
        }
    }

    fn write_track(&mut self, record: &Record) {
        let mut track_file: zip::read::ZipFile = self.zip_file.by_name(&record.filename).unwrap();
        self.kmz_writer
            .start_file("doc.kml", Kmz::default_file_options())
            .unwrap();

        if record.filename.ends_with(".gz") {
            let mut gz_decoder = GzDecoder::new(track_file);
            convert(&mut gz_decoder, &mut self.kmz_writer).unwrap();
        } else {
            convert(&mut track_file, &mut self.kmz_writer).unwrap();
        }
    }

    fn write_medias(&mut self, record: &Record) {
        for media_file_name in &record.medias {
            self.kmz_writer
                .start_file(media_file_name, Kmz::default_file_options())
                .unwrap();
            let mut media_file = self.zip_file.by_name(media_file_name).unwrap();
            copy(&mut media_file, &mut self.kmz_writer).unwrap();
        }
    }

    fn finish(&mut self) {
        self.kmz_writer.finish().unwrap();
    }

    fn convert(kmz_file_name: &str, zip_file: &'a mut zip::ZipArchive<File>, record: &Record) {
        let mut kmz = Kmz::new(&kmz_file_name, zip_file);
        kmz.write_track(&record);
        kmz.write_medias(&record);
        kmz.finish();
    }

    fn default_file_options() -> FileOptions {
        FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755)
    }
}

fn main() {
    let args = Cli::parse();
    let file = fs::File::open(args.in_file).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    let records = extract_records(&mut archive);

    records
        .into_iter()
        .map(|x: Record| Kmz::convert(&format!("{}.kmz", &x.activity_id), &mut archive, &x))
        .for_each(drop);
}
