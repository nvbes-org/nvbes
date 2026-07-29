use std::io::Cursor;

use crate::http::error::AppError;

const MAX_ARCHIVE_ENTRIES: usize = 1_000;
const MAX_ARCHIVE_EXPANDED_BYTES: u64 = 100 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 100;

pub fn validate_content(declared_mime: Option<&str>, data: &[u8]) -> Result<(), AppError> {
    reject_executable(data)?;

    let declared = declared_mime.unwrap_or("application/octet-stream");
    if is_zip(data) {
        validate_zip(data)?;
        if !is_zip_container_mime(declared) {
            return Err(mime_mismatch(declared, "application/zip"));
        }
        return Ok(());
    }

    if let Some(detected) = detect_mime(data)
        && !mime_matches(declared, detected)
    {
        return Err(mime_mismatch(declared, detected));
    }

    if declared == "text/plain" && (data.contains(&0) || std::str::from_utf8(data).is_err()) {
        return Err(AppError::bad_request(
            "upload_content_invalid",
            "Text uploads must contain valid UTF-8 without NUL bytes.",
        ));
    }

    Ok(())
}

fn detect_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"%PDF-") {
        Some("application/pdf")
    } else if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn reject_executable(data: &[u8]) -> Result<(), AppError> {
    let is_mach_o = data.starts_with(&[0xFE, 0xED, 0xFA, 0xCE])
        || data.starts_with(&[0xFE, 0xED, 0xFA, 0xCF])
        || data.starts_with(&[0xCF, 0xFA, 0xED, 0xFE])
        || data.starts_with(&[0xCE, 0xFA, 0xED, 0xFE]);
    if data.starts_with(b"MZ") || data.starts_with(&[0x7F, b'E', b'L', b'F']) || is_mach_o {
        return Err(AppError::bad_request(
            "upload_executable_forbidden",
            "Executable files are not accepted.",
        ));
    }
    Ok(())
}

fn validate_zip(data: &[u8]) -> Result<(), AppError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(|_| {
        AppError::bad_request("upload_archive_invalid", "The ZIP archive is malformed.")
    })?;

    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(AppError::bad_request(
            "upload_archive_too_complex",
            "The ZIP archive contains too many entries.",
        ));
    }

    let mut expanded_bytes = 0_u64;
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|_| {
            AppError::bad_request("upload_archive_invalid", "The ZIP archive is malformed.")
        })?;
        expanded_bytes = expanded_bytes.saturating_add(file.size());
        if expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err(AppError::bad_request(
                "upload_archive_expansion_limit",
                "The expanded ZIP archive exceeds the processing limit.",
            ));
        }

        let compressed = file.compressed_size();
        if compressed > 0 && file.size() / compressed > MAX_COMPRESSION_RATIO {
            return Err(AppError::bad_request(
                "upload_archive_ratio_limit",
                "The ZIP archive compression ratio exceeds the safety limit.",
            ));
        }
    }

    Ok(())
}

fn is_zip(data: &[u8]) -> bool {
    data.starts_with(b"PK\x03\x04")
        || data.starts_with(b"PK\x05\x06")
        || data.starts_with(b"PK\x07\x08")
}

fn is_zip_container_mime(mime: &str) -> bool {
    matches!(
        mime,
        "application/zip"
            | "application/x-zip-compressed"
            | "application/epub+zip"
            | "application/vnd.oasis.opendocument.text"
            | "application/vnd.oasis.opendocument.spreadsheet"
            | "application/vnd.oasis.opendocument.presentation"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    )
}

fn mime_matches(declared: &str, detected: &str) -> bool {
    declared == detected || declared == "application/octet-stream"
}

fn mime_mismatch(declared: &str, detected: &str) -> AppError {
    AppError::bad_request(
        "upload_mime_mismatch",
        format!("Declared MIME type {declared} does not match detected content type {detected}."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_mime_spoofing() {
        let result = validate_content(Some("image/png"), b"%PDF-1.7\n");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_executables_even_as_octet_stream() {
        let result = validate_content(Some("application/octet-stream"), b"MZpayload");
        assert!(result.is_err());
    }

    #[test]
    fn accepts_matching_png_magic_bytes() {
        let bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(validate_content(Some("image/png"), &bytes).is_ok());
    }

    #[test]
    fn rejects_high_ratio_archives() {
        let mut bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
            writer
                .start_file::<_, ()>(
                    "payload.txt",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .unwrap();
            writer.write_all(&vec![b'A'; 2 * 1024 * 1024]).unwrap();
            writer.finish().unwrap();
        }

        assert!(validate_content(Some("application/zip"), &bytes).is_err());
    }
}
