use chrono::{NaiveDate, Utc};

use crate::{
    error::AppError,
    profile_models::{UpdateProfileInput, ValidatedProfileUpdate},
};

pub(crate) fn validate(input: UpdateProfileInput) -> Result<ValidatedProfileUpdate, AppError> {
    let firstname = optional_text("firstname", input.firstname, 100)?;
    let lastname = optional_text("lastname", input.lastname, 100)?;
    let username = optional_text("username", input.username, 100)?;
    let region = optional_text("region", input.region, 64)?;
    let birthdate = input
        .birthdate
        .map(|value| parse_birthdate(&value))
        .transpose()?;

    Ok(ValidatedProfileUpdate {
        firstname,
        lastname,
        username,
        birthdate,
        region,
    })
}

fn optional_text(
    field: &'static str,
    value: Option<String>,
    max_chars: usize,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| {
            let normalized = value.trim();
            if normalized.is_empty() || normalized.chars().count() > max_chars {
                return Err(AppError::bad_request(
                    "invalid_profile",
                    format!("`{field}` must contain between 1 and {max_chars} characters."),
                ));
            }
            Ok(normalized.to_string())
        })
        .transpose()
}

fn parse_birthdate(value: &str) -> Result<NaiveDate, AppError> {
    let date = NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| {
        AppError::bad_request(
            "invalid_profile",
            "`birthdate` must use the YYYY-MM-DD format.",
        )
    })?;
    let earliest = NaiveDate::from_ymd_opt(1900, 1, 1).expect("valid static date");
    if date < earliest || date > Utc::now().date_naive() {
        return Err(AppError::bad_request(
            "invalid_profile",
            "`birthdate` must be between 1900-01-01 and today.",
        ));
    }
    Ok(date)
}

#[cfg(test)]
mod tests {
    use crate::profile_models::UpdateProfileInput;

    use super::validate;

    #[test]
    fn profile_text_is_trimmed_and_birthdate_is_validated() {
        let update = validate(UpdateProfileInput {
            firstname: Some(" Ada ".to_string()),
            lastname: None,
            username: None,
            birthdate: Some("1815-12-10".to_string()),
            region: None,
        });
        assert!(update.is_err());

        let update = validate(UpdateProfileInput {
            firstname: Some(" Ada ".to_string()),
            lastname: None,
            username: None,
            birthdate: Some("1990-12-10".to_string()),
            region: None,
        })
        .expect("valid profile");
        assert_eq!(update.firstname.as_deref(), Some("Ada"));
    }
}
