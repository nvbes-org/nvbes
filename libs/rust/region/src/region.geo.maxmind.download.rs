use std::io::{Cursor, Read};

use reqwest::Client;
use zip::ZipArchive;

use super::maxmind_types::MaxMindGeoLiteConfig;
use super::maxmind_types::MaxMindGeoLiteRange;

pub const GEOLITE_DOWNLOAD_URL: &str = "https://download.maxmind.com/app/geoip_download";

pub fn count_ranges(ranges: &[MaxMindGeoLiteRange], source_code: &str) -> u64 {
    ranges
        .iter()
        .filter(|range| range.source_code == source_code)
        .count() as u64
}

pub async fn download_csv_archive(
    client: &Client,
    config: &MaxMindGeoLiteConfig,
    edition_id: &'static str,
) -> Result<Vec<u8>, reqwest::Error> {
    Ok(client
        .get(GEOLITE_DOWNLOAD_URL)
        .query(&[
            ("edition_id", edition_id),
            ("license_key", config.license_key.as_str()),
            ("suffix", "zip"),
        ])
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

pub fn unzip_archive(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, MaxMindGeoLiteDownloadError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))?;
    let mut files = Vec::new();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        files.push((file.name().to_string(), contents));
    }

    Ok(files)
}

#[derive(Debug, thiserror::Error)]
pub enum MaxMindGeoLiteDownloadError {
    #[error("MaxMind GeoLite zip extraction failed")]
    Zip(#[from] zip::result::ZipError),
    #[error("MaxMind GeoLite archive contains an unreadable file")]
    Io(#[from] std::io::Error),
}
