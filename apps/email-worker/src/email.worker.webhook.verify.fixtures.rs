use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    pkey::{PKey, Private},
    rsa::Rsa,
    x509::{X509, X509NameBuilder, extension::BasicConstraints},
};

pub fn certificate_authority(common_name: &str) -> (X509, PKey<Private>) {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let name = name(common_name);
    let mut builder = base_certificate(&name, &name, &key);
    builder
        .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
        .unwrap();
    builder.sign(&key, MessageDigest::sha256()).unwrap();
    (builder.build(), key)
}

pub fn leaf_certificate(
    common_name: &str,
    issuer: &X509,
    issuer_key: &PKey<Private>,
) -> (X509, PKey<Private>) {
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let subject = name(common_name);
    let mut builder = base_certificate(&subject, issuer.subject_name(), &key);
    builder.sign(issuer_key, MessageDigest::sha256()).unwrap();
    (builder.build(), key)
}

fn base_certificate(
    subject: &openssl::x509::X509NameRef,
    issuer: &openssl::x509::X509NameRef,
    key: &PKey<Private>,
) -> openssl::x509::X509Builder {
    let mut builder = X509::builder().unwrap();
    let mut serial = BigNum::new().unwrap();
    serial.rand(128, MsbOption::MAYBE_ZERO, false).unwrap();
    builder
        .set_serial_number(&serial.to_asn1_integer().unwrap())
        .unwrap();
    builder.set_version(2).unwrap();
    builder.set_subject_name(subject).unwrap();
    builder.set_issuer_name(issuer).unwrap();
    builder.set_pubkey(key).unwrap();
    builder
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    builder
        .set_not_after(&Asn1Time::days_from_now(30).unwrap())
        .unwrap();
    builder
}

fn name(common_name: &str) -> openssl::x509::X509Name {
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", common_name).unwrap();
    name.build()
}
