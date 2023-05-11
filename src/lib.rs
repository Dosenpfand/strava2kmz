use anyhow::{Context, Result};

use flate2::read::GzDecoder;
use gpx_kml_convert::convert;
use serde::{Deserialize, Deserializer};
use std::{fs::File, io::copy, result};
use zip::write::FileOptions;

const ACTIVITIES_FILE_NAME: &str = "activities.csv";
const KML_FILE_NAME: &str = "doc.kml";
const GZIP_FILE_EXTENSION: &str = ".gz";

#[derive(Debug, Deserialize)]
pub struct Record {
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

    pub fn activity_id(&self) -> &str {
        self.activity_id.as_ref()
    }
}

pub fn extract_records(zip_file: &mut zip::ZipArchive<File>) -> Result<Vec<Record>> {
    let activities_file = zip_file.by_name(ACTIVITIES_FILE_NAME)?;
    let mut rdr = csv::Reader::from_reader(activities_file);
    let records: Result<Vec<Record>, csv::Error> = rdr.deserialize().collect();
    Ok(records?)
}

pub struct Kmz<'a> {
    kmz_writer: zip::ZipWriter<File>,
    zip_file: &'a mut zip::ZipArchive<File>,
}

impl<'a> Kmz<'a> {
    pub fn new(kmz_file_name: &str, zip_file: &'a mut zip::ZipArchive<File>) -> Result<Kmz<'a>> {
        let kmz_path = std::path::Path::new(kmz_file_name);
        let kmz_file = std::fs::File::create(kmz_path)?;
        let kmz_writer = zip::ZipWriter::new(kmz_file);
        Ok(Kmz {
            kmz_writer,
            zip_file,
        })
    }

    pub fn write_track(&mut self, record: &Record) -> Result<()> {
        let mut track_file: zip::read::ZipFile = self.zip_file.by_name(&record.filename)?;
        self.kmz_writer
            .start_file(KML_FILE_NAME, Kmz::default_file_options())?;

        if record.filename.ends_with(GZIP_FILE_EXTENSION) {
            let mut gz_decoder = GzDecoder::new(track_file);
            convert(&mut gz_decoder, &mut self.kmz_writer)?;
        } else {
            convert(&mut track_file, &mut self.kmz_writer)?;
        }
        Ok(())
    }

    pub fn write_medias(&mut self, record: &Record) -> Result<()> {
        for media_file_name in &record.medias {
            self.kmz_writer
                .start_file(media_file_name, Kmz::default_file_options())?;
            let mut media_file = self.zip_file.by_name(media_file_name)?;
            copy(&mut media_file, &mut self.kmz_writer)?;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.kmz_writer.finish()?;
        Ok(())
    }

    pub fn convert(
        kmz_file_name: &str,
        zip_file: &'a mut zip::ZipArchive<File>,
        record: &Record,
    ) -> Result<()> {
        let mut kmz = Kmz::new(kmz_file_name, zip_file)
            .with_context(|| format!("Could not create kmz for {}", kmz_file_name))?;
        kmz.write_track(record)
            .with_context(|| format!("Could not write track for {}", kmz_file_name))?;
        kmz.write_medias(record)
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
