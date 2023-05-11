use anyhow::Result;
use serde::{Deserialize, Deserializer};
use std::{io::Read, result};

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

    pub fn extract_records<R: Read>(activities_file: R) -> Result<Vec<Activity>> {
        let mut rdr = csv::Reader::from_reader(activities_file);
        let records: Result<Vec<Activity>, csv::Error> = rdr.deserialize().collect();
        Ok(records?)
    }

    pub fn activity_id(&self) -> &str {
        self.activity_id.as_ref()
    }

    pub fn filename(&self) -> &str {
        self.filename.as_ref()
    }

    pub fn medias(&self) -> &[String] {
        self.medias.as_ref()
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
