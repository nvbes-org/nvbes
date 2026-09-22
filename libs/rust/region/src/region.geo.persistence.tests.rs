use super::GeoLookupPurpose;

#[test]
fn geo_lookup_purpose_matches_database_labels() {
    let purposes = [
        (GeoLookupPurpose::Payment, "payment"),
        (GeoLookupPurpose::Security, "security"),
        (GeoLookupPurpose::DataRegion, "data_region"),
        (GeoLookupPurpose::Auth, "auth"),
        (GeoLookupPurpose::Audit, "audit"),
        (GeoLookupPurpose::DriveApi, "drive_api"),
        (GeoLookupPurpose::DriveAudit, "drive_audit"),
    ];

    for (purpose, database_label) in purposes {
        assert_eq!(purpose.as_str(), database_label);
    }
}
