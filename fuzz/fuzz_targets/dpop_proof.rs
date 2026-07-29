#![no_main]

use libfuzzer_sys::fuzz_target;
use nvbes_dpop::verify_dpop_proof;

fuzz_target!(|data: &[u8]| {
    let proof = String::from_utf8_lossy(data);
    let _ = verify_dpop_proof(
        &proof,
        "POST",
        "https://identity.nvbes.fr/oauth/token",
        None,
        300,
    );
});
