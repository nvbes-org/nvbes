#![no_main]

use libfuzzer_sys::fuzz_target;
use nvbes_account_service::domains::oauth::rar::parse_authorization_details;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = parse_authorization_details(Some(&input));
});
