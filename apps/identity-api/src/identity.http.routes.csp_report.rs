use axum::{body::Bytes, http::StatusCode, response::IntoResponse};

pub async fn csp_report_handler(body: Bytes) -> impl IntoResponse {
    if let Ok(report) = serde_json::from_slice::<serde_json::Value>(&body)
        && let Some(csp_report) = report.get("csp-report")
    {
        tracing::warn!(
            blocked_uri = csp_report
                .get("blocked-uri")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            document_uri = csp_report
                .get("document-uri")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            violated_directive = csp_report
                .get("violated-directive")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            effective_directive = csp_report
                .get("effective-directive")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            source_file = csp_report
                .get("source-file")
                .and_then(|v| v.as_str())
                .unwrap_or_default(),
            "CSP violation reported",
        );
    }

    StatusCode::NO_CONTENT
}
