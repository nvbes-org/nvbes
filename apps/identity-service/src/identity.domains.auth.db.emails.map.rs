use chrono::{DateTime, Utc};
use sqlx::Row;

use crate::domains::auth::types::EmailAddressView;
use crate::http::error::AppError;

pub fn email_address_view(row: sqlx::postgres::PgRow) -> EmailAddressView {
    EmailAddressView {
        id: row.get("id"),
        email: row.get("email"),
        is_primary: row.get("is_primary"),
        verified: row.get::<Option<DateTime<Utc>>, _>("verified_at").is_some(),
        verified_at: row.get("verified_at"),
        created_at: row.get("created_at"),
    }
}

pub fn email_constraint_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(ref db_err) = error {
        email_constraint_error_for(db_err.constraint())
    } else {
        error.into()
    }
}

pub(crate) fn email_constraint_error_for(constraint: Option<&str>) -> AppError {
    match constraint {
        Some("idx_user_email_addresses_active_normalized") | Some("users_email_key") => {
            AppError::conflict(
                "email_already_exists",
                "This email is already associated with an account.",
            )
        }
        Some("idx_user_email_addresses_one_primary") => AppError::conflict(
            "primary_email_conflict",
            "This account already has a primary email.",
        ),
        _ => AppError::internal("database_error", "Failed to update email addresses."),
    }
}

#[cfg(test)]
mod tests {
    use super::email_constraint_error_for;

    #[test]
    fn users_email_constraint_is_a_business_conflict() {
        let app_error = email_constraint_error_for(Some("users_email_key"));

        assert_eq!(app_error.code, "email_already_exists");
        assert_eq!(app_error.status, axum::http::StatusCode::CONFLICT);
    }
}
