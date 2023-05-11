use anyhow::{Context, Result};

use flate2::read::GzDecoder;
use gpx_kml_convert::convert;
use serde::{Deserialize, Deserializer};
use std::{
    fs::File,
    io::{copy, Read},
    result,
};
use zip::write::FileOptions;

const KML_FILE_NAME: &str = "doc.kml";
const GZIP_FILE_EXTENSION: &str = ".gz";

#[derive(Debug, Deserialize)]
pub struct Activity {
    // TODO: string slices?
    #[serde(rename(deserialize = "Activity ID"))]
    activity_id: String,
    #[serde(rename(deserialize = "Filename"))]
    filename: String,
    #[serde(
        rename(deserialize = "Media"),
        deserialize_with = "Activity::deserialize_media"
    )]
    medias: Vec<String>,
}

impl Activity {
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

    pub fn extract_records<R: Read>(activities_file: R) -> Result<Vec<Activity>> {
        let mut rdr = csv::Reader::from_reader(activities_file);
        let records: Result<Vec<Activity>, csv::Error> = rdr.deserialize().collect();
        Ok(records?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_record_single_media() {
        let csv = "Activity ID,Filename,Media\n\
                         123,activities/123.gpx,media/456.jpg";
        let records = Activity::extract_records(&mut csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].activity_id, "123");
        assert_eq!(records[0].filename, "activities/123.gpx");
        assert_eq!(records[0].medias, vec!["media/456.jpg"]);
    }
    #[test]
    fn test_single_record_no_media() {
        let csv = "Activity ID,Filename,Media\n\
                         123,activities/123.gpx,";
        let records = Activity::extract_records(&mut csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].activity_id, "123");
        assert_eq!(records[0].filename, "activities/123.gpx");
        assert_eq!(records[0].medias, Vec::<String>::new());
    }
    #[test]
    fn test_single_record_multiple_media() {
        let csv = "Activity ID,Filename,Media\n\
                         123,activities/123.gpx,media/456.jpg|media/789.jpg";
        let records = Activity::extract_records(&mut csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].activity_id, "123");
        assert_eq!(records[0].filename, "activities/123.gpx");
        assert_eq!(records[0].medias, vec!["media/456.jpg", "media/789.jpg"]);
    }
    #[test]
    fn test_multiple_record() {
        let csv = "Activity ID,Filename,Media\n\
                         123,activities/123.gpx,media/456.jpg\n\
                         123,activities/123.gpx,media/456.jpg\n\
                         123,activities/123.gpx,media/456.jpg";
        let records = Activity::extract_records(&mut csv.as_bytes()).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].activity_id, "123");
        assert_eq!(records[0].filename, "activities/123.gpx");
        assert_eq!(records[0].medias, vec!["media/456.jpg"]);
        assert_eq!(records[1].activity_id, "123");
        assert_eq!(records[1].filename, "activities/123.gpx");
        assert_eq!(records[1].medias, vec!["media/456.jpg"]);
        assert_eq!(records[2].activity_id, "123");
        assert_eq!(records[2].filename, "activities/123.gpx");
        assert_eq!(records[2].medias, vec!["media/456.jpg"]);
    }
}

pub struct KmzConverter<'a> {
    kmz_writer: zip::ZipWriter<File>,
    zip_file: &'a mut zip::ZipArchive<File>,
}

impl<'a> KmzConverter<'a> {
    pub fn new(
        kmz_file_name: &str,
        zip_file: &'a mut zip::ZipArchive<File>,
    ) -> Result<KmzConverter<'a>> {
        let kmz_path = std::path::Path::new(kmz_file_name);
        let kmz_file = std::fs::File::create(kmz_path)?;
        let kmz_writer = zip::ZipWriter::new(kmz_file);
        Ok(KmzConverter {
            kmz_writer,
            zip_file,
        })
    }

    pub fn write_track(&mut self, record: &Activity) -> Result<()> {
        let mut track_file: zip::read::ZipFile = self.zip_file.by_name(&record.filename)?;
        self.kmz_writer
            .start_file(KML_FILE_NAME, KmzConverter::default_file_options())?;

        if record.filename.ends_with(GZIP_FILE_EXTENSION) {
            let mut gz_decoder = GzDecoder::new(track_file);
            convert(&mut gz_decoder, &mut self.kmz_writer)?;
        } else {
            convert(&mut track_file, &mut self.kmz_writer)?;
        }
        Ok(())
    }

    pub fn write_medias(&mut self, record: &Activity) -> Result<()> {
        for media_file_name in &record.medias {
            self.kmz_writer
                .start_file(media_file_name, KmzConverter::default_file_options())?;
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
        record: &Activity,
    ) -> Result<()> {
        let mut kmz = KmzConverter::new(kmz_file_name, zip_file)
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
