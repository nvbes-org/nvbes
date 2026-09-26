use std::collections::HashMap;

use thiserror::Error;

use super::maxmind_types::{
    MAXMIND_GEOLITE_ASN_SOURCE_CODE, MAXMIND_GEOLITE_CITY_SOURCE_CODE,
    MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE, MaxMindGeoLiteRange,
};
use crate::geo::GeoLocation;

pub const GEOLITE_COUNTRY_CSV_EDITION: &str = "GeoLite2-Country-CSV";
pub const GEOLITE_CITY_CSV_EDITION: &str = "GeoLite2-City-CSV";
pub const GEOLITE_ASN_CSV_EDITION: &str = "GeoLite2-ASN-CSV";

#[derive(Debug, Error)]
pub enum MaxMindGeoLiteCsvError {
    #[error("MaxMind GeoLite csv parse failed")]
    Csv(#[from] csv::Error),
    #[error("MaxMind GeoLite csv is missing required file: {0}")]
    MissingFile(&'static str),
}

pub fn parse_geolite_csv_archives(
    country_files: &[(String, Vec<u8>)],
    city_files: &[(String, Vec<u8>)],
    asn_files: &[(String, Vec<u8>)],
) -> Result<Vec<MaxMindGeoLiteRange>, MaxMindGeoLiteCsvError> {
    let mut ranges = parse_geo_archive(
        country_files,
        GEOLITE_COUNTRY_CSV_EDITION,
        MAXMIND_GEOLITE_COUNTRY_SOURCE_CODE,
        "Country",
    )?;
    ranges.extend(parse_geo_archive(
        city_files,
        GEOLITE_CITY_CSV_EDITION,
        MAXMIND_GEOLITE_CITY_SOURCE_CODE,
        "City",
    )?);
    ranges.extend(parse_asn_archive(asn_files)?);
    Ok(ranges)
}

fn parse_geo_archive(
    files: &[(String, Vec<u8>)],
    edition_id: &'static str,
    source_code: &'static str,
    kind: &'static str,
) -> Result<Vec<MaxMindGeoLiteRange>, MaxMindGeoLiteCsvError> {
    let (locations_suffix, ipv4_suffix, ipv6_suffix) = geo_suffixes(kind);
    let countries = parse_locations(find_file(files, locations_suffix)?)?;
    let mut ranges = parse_geo_blocks(
        find_file(files, ipv4_suffix)?,
        &countries,
        edition_id,
        source_code,
    )?;
    ranges.extend(parse_geo_blocks(
        find_file(files, ipv6_suffix)?,
        &countries,
        edition_id,
        source_code,
    )?);
    Ok(ranges)
}

fn geo_suffixes(kind: &'static str) -> (&'static str, &'static str, &'static str) {
    match kind {
        "City" => (
            "GeoLite2-City-Locations-en.csv",
            "GeoLite2-City-Blocks-IPv4.csv",
            "GeoLite2-City-Blocks-IPv6.csv",
        ),
        _ => (
            "GeoLite2-Country-Locations-en.csv",
            "GeoLite2-Country-Blocks-IPv4.csv",
            "GeoLite2-Country-Blocks-IPv6.csv",
        ),
    }
}

fn parse_asn_archive(
    files: &[(String, Vec<u8>)],
) -> Result<Vec<MaxMindGeoLiteRange>, MaxMindGeoLiteCsvError> {
    let mut ranges = parse_asn_blocks(find_file(files, "GeoLite2-ASN-Blocks-IPv4.csv")?)?;
    ranges.extend(parse_asn_blocks(find_file(
        files,
        "GeoLite2-ASN-Blocks-IPv6.csv",
    )?)?);
    Ok(ranges)
}

fn find_file<'a>(
    files: &'a [(String, Vec<u8>)],
    suffix: &'static str,
) -> Result<&'a [u8], MaxMindGeoLiteCsvError> {
    files
        .iter()
        .find(|(name, _)| name.ends_with(suffix))
        .map(|(_, bytes)| bytes.as_slice())
        .ok_or(MaxMindGeoLiteCsvError::MissingFile(suffix))
}

fn parse_locations(bytes: &[u8]) -> Result<HashMap<String, String>, MaxMindGeoLiteCsvError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers()?.clone();
    let geoname_idx = header_index(&headers, "geoname_id")?;
    let country_idx = header_index(&headers, "country_iso_code")?;
    let mut countries = HashMap::new();

    for record in reader.records() {
        let record = record?;
        let geoname_id = record.get(geoname_idx).unwrap_or("").trim();
        let country_code = record
            .get(country_idx)
            .unwrap_or("")
            .trim()
            .to_ascii_uppercase();
        if geoname_id.is_empty() || GeoLocation::from_country_code(&country_code).is_none() {
            continue;
        }
        countries.insert(geoname_id.to_string(), country_code);
    }
    Ok(countries)
}

fn parse_geo_blocks(
    bytes: &[u8],
    countries: &HashMap<String, String>,
    edition_id: &'static str,
    source_code: &'static str,
) -> Result<Vec<MaxMindGeoLiteRange>, MaxMindGeoLiteCsvError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers()?.clone();
    let network_idx = header_index(&headers, "network")?;
    let geoname_idx = header_index(&headers, "geoname_id")?;
    let registered_idx = header_index(&headers, "registered_country_geoname_id")?;
    let mut ranges = Vec::new();

    for record in reader.records() {
        let record = record?;
        let geoname_id = first_non_empty(&record, &[geoname_idx, registered_idx]);
        let Some(country_code) = geoname_id.and_then(|id| countries.get(id)) else {
            continue;
        };
        let Some(network) = record.get(network_idx).and_then(|value| value.parse().ok()) else {
            continue;
        };
        ranges.push(MaxMindGeoLiteRange {
            source_code,
            edition_id,
            network,
            country_code: Some(country_code.clone()),
            geoname_id: geoname_id.map(str::to_string),
            asn: None,
            organization: None,
        });
    }
    Ok(ranges)
}

fn parse_asn_blocks(bytes: &[u8]) -> Result<Vec<MaxMindGeoLiteRange>, MaxMindGeoLiteCsvError> {
    let mut reader = csv::Reader::from_reader(bytes);
    let headers = reader.headers()?.clone();
    let network_idx = header_index(&headers, "network")?;
    let asn_idx = header_index(&headers, "autonomous_system_number")?;
    let org_idx = header_index(&headers, "autonomous_system_organization")?;
    let mut ranges = Vec::new();

    for record in reader.records() {
        let record = record?;
        let Some(network) = record.get(network_idx).and_then(|value| value.parse().ok()) else {
            continue;
        };
        ranges.push(MaxMindGeoLiteRange {
            source_code: MAXMIND_GEOLITE_ASN_SOURCE_CODE,
            edition_id: GEOLITE_ASN_CSV_EDITION,
            network,
            country_code: None,
            geoname_id: None,
            asn: record.get(asn_idx).and_then(|value| value.parse().ok()),
            organization: record
                .get(org_idx)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
        });
    }
    Ok(ranges)
}

fn header_index(
    headers: &csv::StringRecord,
    name: &'static str,
) -> Result<usize, MaxMindGeoLiteCsvError> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or(MaxMindGeoLiteCsvError::MissingFile(name))
}

fn first_non_empty<'a>(record: &'a csv::StringRecord, indexes: &[usize]) -> Option<&'a str> {
    indexes
        .iter()
        .filter_map(|index| record.get(*index).map(str::trim))
        .find(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "region.geo.maxmind.csv.tests.rs"]
mod tests;
