use sqlx::Row;

use crate::domains::auth::types::MfaFactorView;

pub fn map_factor_view(row: sqlx::postgres::PgRow) -> MfaFactorView {
    MfaFactorView {
        id: row.get("id"),
        factor_type: row.get("factor_type"),
        kind: row.try_get::<Option<String>, _>("kind").ok().flatten(),
        status: row.get("status"),
        label: row.get("label"),
        created_at: row.get("created_at"),
        confirmed_at: row.get("confirmed_at"),
        last_used_at: row.get("last_used_at"),
    }
}
