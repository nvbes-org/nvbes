use sentry::protocol::Log;

use super::{scrub_sensitive_string, scrub_value_for_key};

pub(crate) fn scrub_log(mut log: Log) -> Option<Log> {
    log.body = scrub_sensitive_string(&log.body);

    for (key, attribute) in &mut log.attributes {
        scrub_value_for_key(key, &mut attribute.0);
    }

    Some(log)
}
