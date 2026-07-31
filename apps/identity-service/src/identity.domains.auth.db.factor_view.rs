use sqlx::Row;

use crate::domains::auth::types::MfaFactorView;

pub fn map_factor_view(row: sqlx::postgres::PgRow) -> MfaFactorView {
    MfaFactorView {
        id: row.get("id"),
        factor_type: row.get("factor_type"),
        kind: row.try_get::<Option<String>, _>("kind").ok().flatten(),
        assurance: row
            .try_get::<Option<String>, _>("webauthn_assurance")
            .ok()
            .flatten(),
        phishing_resistant: row.get::<String, _>("factor_type") == "webauthn",
        backup_eligible: row
            .try_get::<Option<bool>, _>("webauthn_backup_eligible")
            .ok()
            .flatten(),
        backup_state: row
            .try_get::<Option<bool>, _>("webauthn_backup_state")
            .ok()
            .flatten(),
        sign_count: row
            .try_get::<Option<i64>, _>("webauthn_sign_count")
            .ok()
            .flatten(),
        attestation_format: row
            .try_get::<Option<String>, _>("webauthn_attestation_format")
            .ok()
            .flatten(),
        status: row.get("status"),
        label: row.get("label"),
        created_at: row.get("created_at"),
        confirmed_at: row.get("confirmed_at"),
        last_used_at: row.get("last_used_at"),
    }
}
