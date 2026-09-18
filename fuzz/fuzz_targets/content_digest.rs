#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the content-digest header parsing and formatting routines
        let b64 = nvbes_core::http::content_digest::sha256_digest_base64(data);
        let header = nvbes_core::http::content_digest::content_digest_header_value(data);
        let _ = nvbes_core::auth::token_hash(s);
        let _ = nvbes_core::auth::token_hash_b64(s);
        let _ = nvbes_core::auth::unique_slug(s);
        assert!(!b64.is_empty());
        assert!(header.starts_with("sha-256=:"));
    }
});
