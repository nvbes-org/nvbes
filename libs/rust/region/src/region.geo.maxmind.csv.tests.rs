use super::{MaxMindGeoLiteCsvError, parse_geolite_csv_archives};

#[test]
fn parses_country_city_and_asn_archives() {
    let ranges = parse_geolite_csv_archives(
        &archive_files("Country"),
        &archive_files("City"),
        &asn_files(),
    )
    .unwrap();

    assert_eq!(ranges.len(), 6);
    assert!(
        ranges
            .iter()
            .any(|range| range.edition_id == "GeoLite2-City-CSV")
    );
    assert!(ranges.iter().any(|range| range.asn == Some(64500)));
    assert!(
        ranges
            .iter()
            .any(|range| range.organization.as_deref() == Some("Example AS"))
    );
}

#[test]
fn skips_unknown_countries_and_empty_networks() {
    let country = vec![
        (
            "GeoLite2-Country-Locations-en.csv".to_string(),
            b"geoname_id,locale_code,continent_code,country_iso_code\n1,en,EU,XX\n2,en,EU,FR\n"
                .to_vec(),
        ),
        (
            "GeoLite2-Country-Blocks-IPv4.csv".to_string(),
            b"network,geoname_id,registered_country_geoname_id\nnot-a-cidr,2,\n203.0.113.0/24,1,\n198.51.100.0/24,2,\n"
                .to_vec(),
        ),
        (
            "GeoLite2-Country-Blocks-IPv6.csv".to_string(),
            b"network,geoname_id,registered_country_geoname_id\n".to_vec(),
        ),
    ];
    let ranges =
        parse_geolite_csv_archives(&country, &archive_files("City"), &asn_files()).unwrap();
    let country_ranges: Vec<_> = ranges
        .iter()
        .filter(|range| range.edition_id == "GeoLite2-Country-CSV")
        .collect();
    assert_eq!(country_ranges.len(), 1);
    assert_eq!(country_ranges[0].country_code.as_deref(), Some("FR"));
}

#[test]
fn reports_missing_required_csv_files() {
    let err = parse_geolite_csv_archives(&[], &archive_files("City"), &asn_files()).unwrap_err();
    assert!(matches!(
        err,
        MaxMindGeoLiteCsvError::MissingFile("GeoLite2-Country-Locations-en.csv")
    ));
}

fn archive_files(kind: &str) -> Vec<(String, Vec<u8>)> {
    vec![
        (
            format!("GeoLite2-{kind}-Locations-en.csv"),
            b"geoname_id,locale_code,continent_code,country_iso_code\n3017382,en,EU,FR\n".to_vec(),
        ),
        (
            format!("GeoLite2-{kind}-Blocks-IPv4.csv"),
            b"network,geoname_id,registered_country_geoname_id\n203.0.113.0/24,3017382,\n".to_vec(),
        ),
        (
            format!("GeoLite2-{kind}-Blocks-IPv6.csv"),
            b"network,geoname_id,registered_country_geoname_id\n2001:db8::/32,,3017382\n".to_vec(),
        ),
    ]
}

fn asn_files() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "GeoLite2-ASN-Blocks-IPv4.csv".to_string(),
            b"network,autonomous_system_number,autonomous_system_organization\n203.0.113.0/24,64500,Example AS\n".to_vec(),
        ),
        (
            "GeoLite2-ASN-Blocks-IPv6.csv".to_string(),
            b"network,autonomous_system_number,autonomous_system_organization\n2001:db8::/32,64501,Example IPv6 AS\n".to_vec(),
        ),
    ]
}
