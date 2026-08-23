use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AvatarUploadInput {
    pub content_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AvatarUploadResponse {
    pub upload_url: String,
    pub object_key: String,
}

pub(crate) fn validate(input: &AvatarUploadInput) -> Result<(), crate::error::AppError> {
    if !matches!(
        input.content_type.as_str(),
        "image/jpeg" | "image/png" | "image/webp"
    ) || !(1..=5 * 1024 * 1024).contains(&input.size_bytes)
    {
        return Err(crate::error::AppError::bad_request(
            "invalid_avatar",
            "Use a JPEG, PNG, or WebP image of 5 MB maximum.",
        ));
    }
    Ok(())
}

pub(crate) fn object_key(principal_id: uuid::Uuid) -> String {
    format!("account/profile-avatars/{principal_id}/avatar")
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{AvatarUploadInput, object_key, validate};

    #[test]
    fn avatar_key_is_deterministic_and_input_is_bounded() {
        let principal = Uuid::nil();
        assert_eq!(
            object_key(principal),
            "account/profile-avatars/00000000-0000-0000-0000-000000000000/avatar"
        );
        assert!(
            validate(&AvatarUploadInput {
                content_type: "image/svg+xml".to_string(),
                size_bytes: 1,
            })
            .is_err()
        );
        assert!(
            validate(&AvatarUploadInput {
                content_type: "image/png".to_string(),
                size_bytes: 5 * 1024 * 1024 + 1,
            })
            .is_err()
        );
    }
}
