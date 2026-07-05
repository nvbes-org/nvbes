use reqwest::Client;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::loyalsoldier_types::{LOYALSOLDIER_GEOIP_DAT_URL, LOYALSOLDIER_GEOIP_SHA256_URL};

#[derive(Debug, Error)]
pub enum LoyalsoldierGeoIpDownloadError {
    #[error("Loyalsoldier GeoIP download failed")]
    Request(#[from] reqwest::Error),
    #[error("Loyalsoldier GeoIP checksum file is invalid")]
    InvalidChecksum,
    #[error("Loyalsoldier GeoIP checksum mismatch")]
    ChecksumMismatch,
}

pub async fn download_verified_loyalsoldier_geoip(
    client: &Client,
) -> Result<Vec<u8>, LoyalsoldierGeoIpDownloadError> {
    let checksum = download_text(client, LOYALSOLDIER_GEOIP_SHA256_URL).await?;
    let dat = download_bytes(client, LOYALSOLDIER_GEOIP_DAT_URL).await?;
    verify_sha256(&dat, parse_sha256sum(&checksum)?)?;
    Ok(dat)
}

async fn download_text(client: &Client, url: &str) -> Result<String, reqwest::Error> {
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
}

async fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>, reqwest::Error> {
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

fn parse_sha256sum(body: &str) -> Result<[u8; 32], LoyalsoldierGeoIpDownloadError> {
    let hash = body
        .split_whitespace()
        .next()
        .ok_or(LoyalsoldierGeoIpDownloadError::InvalidChecksum)?;
    decode_sha256_hex(hash).ok_or(LoyalsoldierGeoIpDownloadError::InvalidChecksum)
}

fn verify_sha256(bytes: &[u8], expected: [u8; 32]) -> Result<(), LoyalsoldierGeoIpDownloadError> {
    let actual = Sha256::digest(bytes);
    if actual.as_slice() == expected {
        Ok(())
    } else {
        Err(LoyalsoldierGeoIpDownloadError::ChecksumMismatch)
    }
}

fn decode_sha256_hex(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for index in 0..32 {
        bytes[index] = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::{decode_sha256_hex, parse_sha256sum};

    #[test]
    fn parses_sha256sum_file() {
        let checksum =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  geoip.dat";

        assert!(parse_sha256sum(checksum).is_ok());
    }

    #[test]
    fn rejects_invalid_sha256sum() {
        assert!(decode_sha256_hex("abc").is_none());
        assert!(parse_sha256sum("").is_err());
    }
}
