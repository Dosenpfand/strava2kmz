use anyhow::{Context, Result};
use clap::Parser;
use flate2::read::GzDecoder;
use gpx_kml_convert::convert;
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

fn extract_records(zip_file: &mut zip::ZipArchive<File>) -> Result<Vec<Record>> {
    let activities_file = zip_file.by_name("activities.csv")?;
    let mut rdr = csv::Reader::from_reader(activities_file);
    let records = rdr.deserialize().map(|x| x.unwrap()).collect();
    Ok(records)
}

struct Kmz<'a> {
    kmz_writer: zip::ZipWriter<File>,
    zip_file: &'a mut zip::ZipArchive<File>,
}

impl<'a> Kmz<'a> {
    fn new(kmz_file_name: &str, zip_file: &'a mut zip::ZipArchive<File>) -> Result<Kmz<'a>> {
        let kmz_path = std::path::Path::new(kmz_file_name);
        let kmz_file = std::fs::File::create(kmz_path)?;
        let kmz_writer = zip::ZipWriter::new(kmz_file);
        Ok(Kmz {
            kmz_writer,
            zip_file,
        })
    }

    fn write_track(&mut self, record: &Record) -> Result<()> {
        let mut track_file: zip::read::ZipFile = self.zip_file.by_name(&record.filename)?;
        self.kmz_writer
            .start_file("doc.kml", Kmz::default_file_options())?;

        if record.filename.ends_with(".gz") {
            let mut gz_decoder = GzDecoder::new(track_file);
            convert(&mut gz_decoder, &mut self.kmz_writer)?;
        } else {
            convert(&mut track_file, &mut self.kmz_writer)?;
        }
        Ok(())
    }

    fn write_medias(&mut self, record: &Record) -> Result<()> {
        for media_file_name in &record.medias {
            self.kmz_writer
                .start_file(media_file_name, Kmz::default_file_options())?;
            let mut media_file = self.zip_file.by_name(media_file_name)?;
            copy(&mut media_file, &mut self.kmz_writer)?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.kmz_writer.finish()?;
        Ok(())
    }

    fn convert(
        kmz_file_name: &str,
        zip_file: &'a mut zip::ZipArchive<File>,
        record: &Record,
    ) -> Result<()> {
        let mut kmz = Kmz::new(&kmz_file_name, zip_file)
            .with_context(|| format!("Could not create kmz for {}", kmz_file_name))?;
        kmz.write_track(&record)
            .with_context(|| format!("Could not write track for {}", kmz_file_name))?;
        kmz.write_medias(&record)
            .with_context(|| format!("Could not write media for {}", kmz_file_name))?;
        kmz.finish()
            .with_context(|| format!("Could not finish for {}", kmz_file_name))?;
        Ok(())
    }

    fn default_file_options() -> FileOptions {
        FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755)
    }
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
        .map(|x: Record| Kmz::convert(&format!("{}.kmz", &x.activity_id), &mut archive, &x))
        .for_each(drop);
    Ok(())
}
