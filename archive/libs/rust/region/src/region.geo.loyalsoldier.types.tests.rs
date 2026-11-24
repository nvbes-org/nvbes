use ipnet::IpNet;

use super::{category_reputation, loyalsoldier_is_country, map_loyalsoldier_range};
use crate::geo::v2fly_dat::V2flyGeoIpDatEntry;

#[test]
fn maps_country_and_enriched_categories() {
    let country = map_loyalsoldier_range(&V2flyGeoIpDatEntry {
        code: "fr".to_string(),
        network: "203.0.113.0/24".parse::<IpNet>().unwrap(),
    });
    let tor = map_loyalsoldier_range(&V2flyGeoIpDatEntry {
        code: "tor".to_string(),
        network: "198.51.100.0/24".parse::<IpNet>().unwrap(),
    });

    assert_eq!(country.country_code.as_deref(), Some("FR"));
    assert_eq!(country.kind, "unknown");
    assert_eq!(country.score, 35);
    assert!(country.labels.contains(&"country_geoip".to_string()));
    assert!(
        country
            .labels
            .contains(&"source:loyalsoldier_geoip".to_string())
    );
    assert_eq!(tor.country_code, None);
    assert_eq!(tor.kind, "tor");
    assert_eq!(tor.score, 95);
    assert!(loyalsoldier_is_country("de"));
    assert!(!loyalsoldier_is_country("tor"));
}

#[test]
fn classifies_provider_categories_as_datacenter() {
    for code in ["cloudflare", "cloudfront", "fastly", "google"] {
        let (kind, score, labels) = category_reputation(code, false);
        assert_eq!(kind.as_str(), "datacenter");
        assert_eq!(score, 65);
        assert!(labels.contains(&format!("provider:{code}")));
    }
    for code in ["facebook", "netflix", "telegram", "twitter"] {
        let (kind, score, labels) = category_reputation(code, false);
        assert_eq!(kind.as_str(), "datacenter");
        assert_eq!(score, 60);
        assert!(labels.contains(&format!("provider:{code}")));
    }
}

#[test]
fn classifies_private_and_unknown_categories() {
    let (kind, score, labels) = category_reputation("private", false);
    assert_eq!(kind.as_str(), "unknown");
    assert_eq!(score, 0);
    assert!(labels.contains(&"private_network".to_string()));

    let (kind, score, labels) = category_reputation("custom-cat", false);
    assert_eq!(kind.as_str(), "unknown");
    assert_eq!(score, 45);
    assert!(labels.contains(&"category:custom-cat".to_string()));
}
