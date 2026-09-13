use chrono::NaiveDate;
use proptest::prelude::*;

use super::{parse_birthdate, validate_birthdate};

proptest! {
    #[test]
    fn arbitrary_raw_input_never_panics(raw in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let input = String::from_utf8_lossy(&raw);
        let _ = parse_birthdate(Some(&input));
    }

    #[test]
    fn round_trip_valid_ymd(
        year in 1900i32..=2099,
        month in 1u32..=12,
        day in 1u32..=28,
    ) {
        let formatted = format!("{year:04}-{month:02}-{day:02}");
        let parsed = parse_birthdate(Some(&formatted)).expect("valid date should parse");

        let expected = NaiveDate::from_ymd_opt(year, month, day);
        prop_assert_eq!(parsed, expected);
    }

    #[test]
    fn validate_birthdate_never_panics(
        year in 1800i32..=2200,
        month in 1u32..=12,
        day in 1u32..=28,
        region in proptest::option::of("[A-Z]{2}"),
    ) {
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            let _ = validate_birthdate(date, region.as_deref());
        }
    }
}
