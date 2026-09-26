use super::{date_in_region_at, parse_birthdate, region_utc_offset, validate_birthdate_on};
use chrono::{DateTime, NaiveDate, Utc};

fn utc(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .expect("fixture timestamp")
        .with_timezone(&Utc)
}

#[test]
fn region_utc_offset_table_covers_every_bucket() {
    let cases = [
        ("FR", 1),
        ("GB", 0),
        ("FI", 2),
        ("BY", 3),
        ("US", -5),
        ("MX", -6),
        ("BR", -3),
        ("AR", -3),
        ("CO", -5),
        ("VE", -4),
        ("JP", 9),
        ("CN", 8),
        ("TH", 7),
        ("IN", 5),
        ("BD", 6),
        ("PK", 5),
        ("IR", 3),
        ("AF", 4),
        ("AU", 10),
        ("NZ", 12),
        ("ZA", 2),
        ("EG", 2),
        ("NG", 1),
        ("KE", 3),
        ("MA", 1),
        ("SA", 3),
        ("AE", 4),
        ("IL", 2),
    ];
    for (code, hours) in cases {
        assert_eq!(region_utc_offset(code), Some(hours), "region {code}");
    }
    assert_eq!(region_utc_offset("ZZ"), None);
}

#[test]
fn region_offsets_shift_local_calendar_day() {
    let near_midnight = utc("2026-06-14T23:30:00Z");
    assert_eq!(
        date_in_region_at(Some("FR"), near_midnight),
        NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()
    );
    assert_eq!(
        date_in_region_at(Some("GB"), near_midnight),
        NaiveDate::from_ymd_opt(2026, 6, 14).unwrap()
    );

    let early = utc("2026-06-15T02:00:00Z");
    assert_eq!(
        date_in_region_at(Some("US"), early),
        NaiveDate::from_ymd_opt(2026, 6, 14).unwrap()
    );
    assert_eq!(
        date_in_region_at(Some("BR"), early),
        NaiveDate::from_ymd_opt(2026, 6, 14).unwrap()
    );
}

#[test]
fn parse_birthdate_enforces_length_and_charset() {
    assert_eq!(parse_birthdate(None).unwrap(), None);
    assert_eq!(parse_birthdate(Some("   ")).unwrap(), None);
    assert!(parse_birthdate(Some(&"2".repeat(65))).is_err());
    assert!(parse_birthdate(Some(&"2".repeat(64))).is_err());
    assert!(parse_birthdate(Some("né-1990-01-01")).is_err());
    assert!(parse_birthdate(Some("1990-01-01\u{0007}")).is_err());
    assert_eq!(
        parse_birthdate(Some("1990-01-01T12:00:00Z")).unwrap(),
        Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap())
    );
}

#[test]
fn validate_birthdate_on_rejects_future_underage_and_overage() {
    let today = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
    assert!(validate_birthdate_on(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(), today).is_ok());
    assert!(validate_birthdate_on(today + chrono::Days::new(1), today).is_err());
    assert!(validate_birthdate_on(NaiveDate::from_ymd_opt(2014, 6, 16).unwrap(), today).is_err());
    assert!(validate_birthdate_on(NaiveDate::from_ymd_opt(2013, 6, 15).unwrap(), today).is_ok());
    assert!(validate_birthdate_on(NaiveDate::from_ymd_opt(1905, 6, 15).unwrap(), today).is_err());
    assert!(validate_birthdate_on(NaiveDate::from_ymd_opt(1899, 1, 1).unwrap(), today).is_err());
}
