use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use super::{MaxMindGeoLiteRange, count_ranges, unzip_archive};

fn sample_range(source_code: &'static str) -> MaxMindGeoLiteRange {
    MaxMindGeoLiteRange {
        source_code,
        edition_id: "fixture",
        network: "203.0.113.0/24".parse().unwrap(),
        country_code: Some("FR".into()),
        geoname_id: None,
        asn: None,
        organization: None,
    }
}

#[test]
fn count_ranges_filters_by_source_code() {
    let ranges = vec![
        sample_range("maxmind_geolite_country_csv"),
        sample_range("maxmind_geolite_country_csv"),
        sample_range("maxmind_geolite_asn_csv"),
    ];
    assert_eq!(count_ranges(&ranges, "maxmind_geolite_country_csv"), 2);
    assert_eq!(count_ranges(&ranges, "maxmind_geolite_asn_csv"), 1);
    assert_eq!(count_ranges(&ranges, "missing"), 0);
}

#[test]
fn unzip_archive_reads_local_zip_fixture() {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("GeoLite2-Country-Locations-en.csv", options)
            .unwrap();
        zip.write_all(b"geoname_id,country_iso_code\n1,FR\n")
            .unwrap();
        zip.start_file("nested/dir/", options).unwrap();
        zip.start_file("nested/blocks.csv", options).unwrap();
        zip.write_all(b"network\n203.0.113.0/24\n").unwrap();
        zip.finish().unwrap();
    }
    let files = unzip_archive(buffer.get_ref()).expect("unzip fixture");
    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .any(|(name, _)| name.ends_with("Locations-en.csv"))
    );
    assert!(files.iter().any(|(name, _)| name.ends_with("blocks.csv")));
}

#[test]
fn unzip_archive_rejects_invalid_bytes() {
    assert!(unzip_archive(b"not-a-zip").is_err());
}

#[tokio::test]
async fn download_csv_archive_reads_zip_bytes_from_mock_server() {
    use reqwest::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::download_csv_archive_from_url;
    use crate::geo::maxmind_types::MaxMindGeoLiteConfig;

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("GeoLite2-Country-Locations-en.csv", options)
            .unwrap();
        zip.write_all(b"geoname_id,country_iso_code\n1,FR\n")
            .unwrap();
        zip.finish().unwrap();
    }

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/app/geoip_download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(buffer.get_ref().clone()))
        .mount(&server)
        .await;

    let config = MaxMindGeoLiteConfig {
        account_id: "account".to_string(),
        license_key: "license".to_string(),
        eula_accepted: true,
    };

    let archive = download_csv_archive_from_url(
        &Client::new(),
        &format!("{}/app/geoip_download", server.uri()),
        &config,
        "GeoLite2-Country-CSV",
    )
    .await
    .expect("download");
    let files = super::unzip_archive(&archive).expect("zip");
    assert!(!files.is_empty());
}
