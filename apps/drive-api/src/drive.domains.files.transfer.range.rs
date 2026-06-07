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
