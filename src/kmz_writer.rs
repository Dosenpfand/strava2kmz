use anyhow::{Context, Result};

use flate2::read::GzDecoder;
use gpx_kml_convert::convert;

use std::{fs::File, io::copy};
use zip::write::FileOptions;

use crate::Activity;

const KML_FILE_NAME: &str = "doc.kml";
const GZIP_FILE_EXTENSION: &str = ".gz";

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
