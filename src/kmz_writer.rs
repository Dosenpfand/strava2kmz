use anyhow::{Context, Result};

use flate2::read::GzDecoder;
use gpx_kml_convert::convert;

use log::warn;
use std::{
    fs::File,
    io::{copy, Read, Seek},
};
use zip::write::FileOptions;

use crate::Activity;

const KML_FILE_NAME: &str = "doc.kml";
const GZIP_FILE_EXTENSION: &str = ".gz";

pub struct KmzConverter<'a, R: Read + Seek> {
    kmz_writer: zip::ZipWriter<File>,
    zip_file: &'a mut zip::ZipArchive<R>,
}

impl<'a, R: Read + Seek> KmzConverter<'a, R> {
    pub fn new(
        kmz_file_name: &str,
        zip_file: &'a mut zip::ZipArchive<R>,
    ) -> Result<KmzConverter<'a, R>> {
        let kmz_path = std::path::Path::new(kmz_file_name);
        let kmz_file = std::fs::File::create(kmz_path)?;
        let kmz_writer = zip::ZipWriter::new(kmz_file);
        Ok(KmzConverter {
            kmz_writer,
            zip_file,
        })
    }

    pub fn write_track(&mut self, record: &Activity) -> Result<()> {
        let mut track_file: zip::read::ZipFile = self.zip_file.by_name(record.filename())?;
        self.kmz_writer
            .start_file(KML_FILE_NAME, default_file_options())?;

        if record.filename().ends_with(GZIP_FILE_EXTENSION) {
            let mut gz_decoder = GzDecoder::new(track_file);
            convert(&mut gz_decoder, &mut self.kmz_writer)?;
        } else {
            convert(&mut track_file, &mut self.kmz_writer)?;
        }
        Ok(())
    }

    pub fn write_medias(&mut self, record: &Activity) -> Result<()> {
        for media_file_name in record.medias() {
            self.kmz_writer
                .start_file(media_file_name, default_file_options())?;
            let media_file_result = self.zip_file.by_name(media_file_name);
            match media_file_result {
                Ok(mut media_file) => {
                    copy(&mut media_file, &mut self.kmz_writer)?;
                }
                Err(_) => {
                    warn!(
                        "Could not find referenced media file '{}' in archive.",
                        media_file_name
                    );
                }
            }
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
}

fn default_file_options() -> FileOptions {
    FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755)
}
