use crate::{
    domains::files::models::{DownloadMetadata, StorageObjectStatus, StorageObjectType},
    http::error::AppError,
};

use super::ResolvedRange;

pub(super) fn ensure_downloadable_file(metadata: &DownloadMetadata) -> Result<(), AppError> {
    if !matches!(metadata.object_type, StorageObjectType::File) {
        return Err(AppError::bad_request(
            "invalid_download_target",
            "Only files can be downloaded.",
        ));
    }

    if matches!(metadata.status, StorageObjectStatus::Quarantined) {
        return Err(AppError::conflict(
            "file_quarantined",
            "This file has been quarantined due to a policy violation.",
        ));
    }

    if !matches!(metadata.status, StorageObjectStatus::Active) {
        return Err(AppError::conflict(
            "object_not_downloadable",
            "Only active files can be downloaded.",
        ));
    }

    Ok(())
}

pub(super) fn resolve_range(
    range_header: Option<&str>,
    size_bytes: i64,
) -> Result<ResolvedRange, AppError> {
    let Some(range_header) = range_header else {
        return Ok(ResolvedRange::Full);
    };

    let range_header = range_header.trim();
    let Some(spec) = range_header.strip_prefix("bytes=") else {
        return Err(AppError::bad_request(
            "invalid_range_header",
            "Range must use the bytes unit.",
        ));
    };

    if spec.contains(',') {
        return Err(AppError::bad_request(
            "multiple_ranges_unsupported",
            "Multiple byte ranges are not supported.",
        ));
    }

    if size_bytes <= 0 {
        return Ok(ResolvedRange::Unsatisfiable);
    }

    let Some((start_raw, end_raw)) = spec.split_once('-') else {
        return Err(AppError::bad_request(
            "invalid_range_header",
            "Range must use start-end syntax.",
        ));
    };

    if start_raw.is_empty() {
        let suffix_len = parse_non_negative_i64(end_raw, "range_suffix")?;
        if suffix_len == 0 {
            return Ok(ResolvedRange::Unsatisfiable);
        }
        let start = (size_bytes - suffix_len).max(0);
        return Ok(ResolvedRange::Partial {
            start,
            end_inclusive: size_bytes - 1,
        });
    }

    let start = parse_non_negative_i64(start_raw, "range_start")?;
    if start >= size_bytes {
        return Ok(ResolvedRange::Unsatisfiable);
    }

    let end_inclusive = if end_raw.is_empty() {
        size_bytes - 1
    } else {
        parse_non_negative_i64(end_raw, "range_end")?.min(size_bytes - 1)
    };

    if end_inclusive < start {
        return Ok(ResolvedRange::Unsatisfiable);
    }

    Ok(ResolvedRange::Partial {
        start,
        end_inclusive,
    })
}

fn parse_non_negative_i64(value: &str, code: &str) -> Result<i64, AppError> {
    let parsed = value.parse::<i64>().map_err(|error| {
        AppError::bad_request(
            code,
            format!("Range value must be a valid integer: {error}"),
        )
    })?;
    if parsed < 0 {
        return Err(AppError::bad_request(
            code,
            "Range value cannot be negative.",
        ));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::{ensure_downloadable_file, resolve_range};
    use crate::domains::files::models::{DownloadMetadata, StorageObjectStatus, StorageObjectType};
    use crate::domains::files::transfer::ResolvedRange;

    fn metadata() -> DownloadMetadata {
        DownloadMetadata {
            object_type: StorageObjectType::File,
            status: StorageObjectStatus::Active,
            object_key: Some("object-key".to_string()),
            size_bytes: 100,
            mime_type: Some("text/plain".to_string()),
        }
    }

    #[test]
    fn ensure_downloadable_file_accepts_active_files_only() {
        assert!(ensure_downloadable_file(&metadata()).is_ok());

        let folder = DownloadMetadata {
            object_type: StorageObjectType::Folder,
            ..metadata()
        };
        assert_eq!(
            ensure_downloadable_file(&folder).unwrap_err().code,
            "invalid_download_target"
        );

        let quarantined = DownloadMetadata {
            status: StorageObjectStatus::Quarantined,
            ..metadata()
        };
        assert_eq!(
            ensure_downloadable_file(&quarantined).unwrap_err().code,
            "file_quarantined"
        );

        let pending = DownloadMetadata {
            status: StorageObjectStatus::Pending,
            ..metadata()
        };
        assert_eq!(
            ensure_downloadable_file(&pending).unwrap_err().code,
            "object_not_downloadable"
        );
    }

    #[test]
    fn resolve_range_supports_full_suffix_and_clamped_ranges() {
        assert!(matches!(
            resolve_range(None, 100).unwrap(),
            ResolvedRange::Full
        ));

        match resolve_range(Some("bytes=-10"), 100).unwrap() {
            ResolvedRange::Partial {
                start,
                end_inclusive,
            } => {
                assert_eq!(start, 90);
                assert_eq!(end_inclusive, 99);
            }
            _ => panic!("expected suffix range"),
        }

        match resolve_range(Some("bytes=90-200"), 100).unwrap() {
            ResolvedRange::Partial {
                start,
                end_inclusive,
            } => {
                assert_eq!(start, 90);
                assert_eq!(end_inclusive, 99);
            }
            _ => panic!("expected clamped range"),
        }
    }

    #[test]
    fn resolve_range_rejects_multiple_ranges_and_marks_unsatisfiable() {
        assert_eq!(
            resolve_range(Some("bytes=0-1,2-3"), 100)
                .err()
                .unwrap()
                .code,
            "multiple_ranges_unsupported"
        );
        assert!(matches!(
            resolve_range(Some("bytes=100-200"), 100).unwrap(),
            ResolvedRange::Unsatisfiable
        ));
    }
}
