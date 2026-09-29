use std::net::IpAddr;

use serde_json::json;

use super::{
    MAXMIND_GEOLITE_SOURCE_CODES, MaxMindGeoLiteConfig, MaxMindGeoLiteConfigError,
    maxmind_ip_relation,
};

#[test]
fn enabled_config_trims_and_requires_eula() {
    let err = MaxMindGeoLiteConfig {
        account_id: "acct".into(),
        license_key: "key".into(),
        eula_accepted: false,
    }
    .enabled()
    .unwrap_err();
    assert_eq!(err, MaxMindGeoLiteConfigError::EulaNotAccepted);

    assert_eq!(
        MaxMindGeoLiteConfig {
            account_id: "  ".into(),
            license_key: "key".into(),
            eula_accepted: true,
        }
        .enabled()
        .unwrap_err(),
        MaxMindGeoLiteConfigError::MissingAccountId
    );
    assert_eq!(
        MaxMindGeoLiteConfig {
            account_id: "acct".into(),
            license_key: " ".into(),
            eula_accepted: true,
        }
        .enabled()
        .unwrap_err(),
        MaxMindGeoLiteConfigError::MissingLicenseKey
    );

    let ok = MaxMindGeoLiteConfig {
        account_id: "  acct  ".into(),
        license_key: "  key  ".into(),
        eula_accepted: true,
    }
    .enabled()
    .unwrap();
    assert_eq!(ok.account_id, "acct");
    assert_eq!(ok.license_key, "key");
}

#[test]
fn maxmind_ip_relation_reads_country_and_traits() {
    let ip: IpAddr = "203.0.113.10".parse().unwrap();
    let payload = json!({
        "country": {"iso_code": "fr"},
        "traits": {
            "autonomous_system_number": 64500,
            "autonomous_system_organization": "Example AS"
        }
    });
    let (relation, location) = maxmind_ip_relation(ip, &payload).unwrap();
    assert_eq!(location.country_code, "FR");
    assert_eq!(relation.asn, Some(64500));
    assert_eq!(relation.organization.as_deref(), Some("Example AS"));
    assert_eq!(relation.network.as_deref(), Some("203.0.113.10/32"));
    assert!(
        relation
            .risk_labels
            .contains(&"source:maxmind_geolite".to_string())
    );
}

#[test]
fn maxmind_ip_relation_falls_back_to_registered_country() {
    let ip: IpAddr = "2001:db8::1".parse().unwrap();
    let payload = json!({
        "registered_country": {"iso_code": "US"}
    });
    let (relation, location) = maxmind_ip_relation(ip, &payload).unwrap();
    assert_eq!(location.country_code, "US");
    assert_eq!(relation.network.as_deref(), Some("2001:db8::1/128"));
}

#[test]
fn maxmind_ip_relation_rejects_unknown_country() {
    let ip: IpAddr = "8.8.8.8".parse().unwrap();
    assert!(maxmind_ip_relation(ip, &json!({"country": {"iso_code": "XX"}})).is_none());
    assert!(maxmind_ip_relation(ip, &json!({})).is_none());
}

#[test]
fn source_codes_cover_csv_and_web() {
    assert_eq!(MAXMIND_GEOLITE_SOURCE_CODES.len(), 4);
    assert!(
        MAXMIND_GEOLITE_SOURCE_CODES
            .iter()
            .any(|code| code.contains("web"))
    );
}
