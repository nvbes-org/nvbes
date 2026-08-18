use chrono::{DateTime, Utc};
use chrono_tz::Tz;

pub(super) struct LocalDateTime {
    pub date: String,
    pub timezone: String,
}

pub(super) fn local_date_time(value: DateTime<Utc>, timezone: &str) -> LocalDateTime {
    let timezone = timezone.parse::<Tz>().unwrap_or(chrono_tz::UTC);
    let local = value.with_timezone(&timezone);
    LocalDateTime {
        date: local.format("%B %-d, %Y at %H:%M").to_string(),
        timezone: format!("{timezone} (UTC{})", local.format("%:z")),
    }
}

pub(super) fn utc_date_time(value: DateTime<Utc>) -> LocalDateTime {
    local_date_time(value, "UTC")
}
