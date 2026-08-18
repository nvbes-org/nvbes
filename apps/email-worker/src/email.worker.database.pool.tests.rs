use std::time::Duration;

use super::DATABASE_ACQUIRE_TIMEOUT;

#[test]
fn acquire_timeout_covers_a_serverless_database_cold_start() {
    assert!(DATABASE_ACQUIRE_TIMEOUT >= Duration::from_secs(10));
}
