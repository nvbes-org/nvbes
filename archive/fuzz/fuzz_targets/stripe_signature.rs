#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    // Split data into secret, header and payload
    let (secret_bytes, rest) = data.split_at(8);
    let (header_bytes, payload) = rest.split_at(rest.len() / 2);

    if let (Ok(secret), Ok(header)) = (std::str::from_utf8(secret_bytes), std::str::from_utf8(header_bytes)) {
        // Must never panic on arbitrary inputs
        let _ = nvbes_billing::verify_stripe_signature(secret, Some(header), payload);
        let _ = nvbes_billing::parse_stripe_event(payload);
    }
});
