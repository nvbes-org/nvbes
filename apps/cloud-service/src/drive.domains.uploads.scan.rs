use crate::http::error::AppError;
use nvbes_scan::{ScanEngine, ScanResult};

pub struct ScanOutcome {
    pub status: String,
    pub is_quarantined: bool,
    pub quarantine_reason: String,
}

pub async fn perform_scan(
    scanner: &dyn ScanEngine,
    file_data: &[u8],
    declared_mime: Option<&str>,
    scan_enabled: bool,
    scan_fail_open: bool,
) -> Result<ScanOutcome, AppError> {
    if let Err(validation_error) =
        super::content_validation::validate_content(declared_mime, file_data)
    {
        tracing::warn!(
            error_code = validation_error.code,
            "File content policy rejected upload"
        );
        return Ok(ScanOutcome {
            status: "content_rejected".to_string(),
            is_quarantined: true,
            quarantine_reason: format!("content_policy:{}", validation_error.code),
        });
    }

    if !scan_enabled {
        return Ok(ScanOutcome {
            status: "unscanned_disabled".to_string(),
            is_quarantined: false,
            quarantine_reason: String::new(),
        });
    }

    match scanner.scan(file_data).await {
        Ok(ScanResult::Clean) => {
            tracing::info!("File scan: clean");
            Ok(ScanOutcome {
                status: "clean".to_string(),
                is_quarantined: false,
                quarantine_reason: String::new(),
            })
        }
        Ok(ScanResult::Infected { signature }) => {
            tracing::warn!(signature = %signature, "File scan: infected");
            Ok(ScanOutcome {
                status: "infected".to_string(),
                is_quarantined: true,
                quarantine_reason: signature,
            })
        }
        Err(scan_error) => {
            tracing::error!(error = %scan_error, "File scan: error");
            if scan_fail_open {
                tracing::warn!("Scan fail-open: activating despite error");
                Ok(ScanOutcome {
                    status: "error_fail_open".to_string(),
                    is_quarantined: false,
                    quarantine_reason: String::new(),
                })
            } else {
                Ok(ScanOutcome {
                    status: "error".to_string(),
                    is_quarantined: true,
                    quarantine_reason: format!("scan_error: {scan_error}"),
                })
            }
        }
    }
}
