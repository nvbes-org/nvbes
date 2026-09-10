#![no_main]

use libfuzzer_sys::fuzz_target;
use nvbes_cloud_service::domains::uploads::logic::{
    normalize_checksum, validate_mime_type, validate_object_name,
};

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = validate_object_name(&input);
    let _ = validate_mime_type(&input);
    let _ = normalize_checksum(Some(input.into_owned()));
});
