use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::Engine;
use hkdf::Hkdf;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const HKDF_INFO: &[u8] = b"nvbes/envelope-encryption/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeEncryptedData {
    pub kek_id: String,
    pub kek_version: u32,
    pub encrypted_dek: String,
    pub dek_nonce: String,
    pub data_nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EnvelopeError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid key or nonce encoding")]
    InvalidEncoding,
    #[error("Master KMS key is too short (min 32 bytes required)")]
    KeyTooShort,
}

pub fn derive_tenant_kek(
    master_kms_key: &[u8],
    tenant_id: &str,
    kek_version: u32,
) -> Result<[u8; KEY_LEN], EnvelopeError> {
    if master_kms_key.len() < KEY_LEN {
        return Err(EnvelopeError::KeyTooShort);
    }
    let salt = format!("{tenant_id}:v{kek_version}");
    let hkdf = Hkdf::<Sha256>::new(Some(salt.as_bytes()), master_kms_key);
    let mut derived = [0u8; KEY_LEN];
    hkdf.expand(HKDF_INFO, &mut derived)
        .map_err(|_| EnvelopeError::EncryptionFailed)?;
    Ok(derived)
}

pub fn encrypt_envelope(
    master_kms_key: &[u8],
    tenant_id: &str,
    kek_version: u32,
    plaintext: &[u8],
) -> Result<EnvelopeEncryptedData, EnvelopeError> {
    let tenant_kek = derive_tenant_kek(master_kms_key, tenant_id, kek_version)?;
    let kek_cipher =
        Aes256Gcm::new_from_slice(&tenant_kek).map_err(|_| EnvelopeError::EncryptionFailed)?;

    let mut dek = [0u8; KEY_LEN];
    rand::rng().fill_bytes(&mut dek);
    let dek_cipher =
        Aes256Gcm::new_from_slice(&dek).map_err(|_| EnvelopeError::EncryptionFailed)?;

    let mut dek_nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut dek_nonce_bytes);
    let dek_nonce = Nonce::from(dek_nonce_bytes);

    let mut data_nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut data_nonce_bytes);
    let data_nonce = Nonce::from(data_nonce_bytes);

    let encrypted_dek_bytes = kek_cipher
        .encrypt(
            &dek_nonce,
            Payload {
                msg: &dek,
                aad: tenant_id.as_bytes(),
            },
        )
        .map_err(|_| EnvelopeError::EncryptionFailed)?;

    let ciphertext_bytes = dek_cipher
        .encrypt(
            &data_nonce,
            Payload {
                msg: plaintext,
                aad: tenant_id.as_bytes(),
            },
        )
        .map_err(|_| EnvelopeError::EncryptionFailed)?;

    let b64 = base64::engine::general_purpose::STANDARD;

    Ok(EnvelopeEncryptedData {
        kek_id: format!("kms:tenant:{tenant_id}"),
        kek_version,
        encrypted_dek: b64.encode(encrypted_dek_bytes),
        dek_nonce: b64.encode(dek_nonce_bytes),
        data_nonce: b64.encode(data_nonce_bytes),
        ciphertext: b64.encode(ciphertext_bytes),
    })
}

pub fn decrypt_envelope(
    master_kms_key: &[u8],
    tenant_id: &str,
    envelope: &EnvelopeEncryptedData,
) -> Result<Vec<u8>, EnvelopeError> {
    let b64 = base64::engine::general_purpose::STANDARD;

    let encrypted_dek_bytes = b64
        .decode(&envelope.encrypted_dek)
        .map_err(|_| EnvelopeError::InvalidEncoding)?;
    let dek_nonce_bytes = b64
        .decode(&envelope.dek_nonce)
        .map_err(|_| EnvelopeError::InvalidEncoding)?;
    let data_nonce_bytes = b64
        .decode(&envelope.data_nonce)
        .map_err(|_| EnvelopeError::InvalidEncoding)?;
    let ciphertext_bytes = b64
        .decode(&envelope.ciphertext)
        .map_err(|_| EnvelopeError::InvalidEncoding)?;

    if dek_nonce_bytes.len() != NONCE_LEN || data_nonce_bytes.len() != NONCE_LEN {
        return Err(EnvelopeError::InvalidEncoding);
    }

    let tenant_kek = derive_tenant_kek(master_kms_key, tenant_id, envelope.kek_version)?;
    let kek_cipher =
        Aes256Gcm::new_from_slice(&tenant_kek).map_err(|_| EnvelopeError::DecryptionFailed)?;

    let dek_nonce = Nonce::from_slice(&dek_nonce_bytes);
    let dek_bytes = kek_cipher
        .decrypt(
            dek_nonce,
            Payload {
                msg: &encrypted_dek_bytes,
                aad: tenant_id.as_bytes(),
            },
        )
        .map_err(|_| EnvelopeError::DecryptionFailed)?;

    if dek_bytes.len() != KEY_LEN {
        return Err(EnvelopeError::DecryptionFailed);
    }

    let dek_cipher =
        Aes256Gcm::new_from_slice(&dek_bytes).map_err(|_| EnvelopeError::DecryptionFailed)?;
    let data_nonce = Nonce::from_slice(&data_nonce_bytes);

    let plaintext = dek_cipher
        .decrypt(
            data_nonce,
            Payload {
                msg: &ciphertext_bytes,
                aad: tenant_id.as_bytes(),
            },
        )
        .map_err(|_| EnvelopeError::DecryptionFailed)?;

    tracing::info!(
        tenant_id = tenant_id,
        kek_version = envelope.kek_version,
        "Audit: Envelope encryption data successfully decrypted"
    );

    Ok(plaintext)
}

pub fn rotate_envelope(
    master_kms_key: &[u8],
    tenant_id: &str,
    new_kek_version: u32,
    envelope: &EnvelopeEncryptedData,
) -> Result<EnvelopeEncryptedData, EnvelopeError> {
    let b64 = base64::engine::general_purpose::STANDARD;

    let encrypted_dek_bytes = b64
        .decode(&envelope.encrypted_dek)
        .map_err(|_| EnvelopeError::InvalidEncoding)?;
    let dek_nonce_bytes = b64
        .decode(&envelope.dek_nonce)
        .map_err(|_| EnvelopeError::InvalidEncoding)?;

    if dek_nonce_bytes.len() != NONCE_LEN {
        return Err(EnvelopeError::InvalidEncoding);
    }

    let current_kek = derive_tenant_kek(master_kms_key, tenant_id, envelope.kek_version)?;
    let current_kek_cipher =
        Aes256Gcm::new_from_slice(&current_kek).map_err(|_| EnvelopeError::DecryptionFailed)?;

    let dek_bytes = current_kek_cipher
        .decrypt(
            Nonce::from_slice(&dek_nonce_bytes),
            Payload {
                msg: &encrypted_dek_bytes,
                aad: tenant_id.as_bytes(),
            },
        )
        .map_err(|_| EnvelopeError::DecryptionFailed)?;

    let new_kek = derive_tenant_kek(master_kms_key, tenant_id, new_kek_version)?;
    let new_kek_cipher =
        Aes256Gcm::new_from_slice(&new_kek).map_err(|_| EnvelopeError::EncryptionFailed)?;

    let mut new_dek_nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut new_dek_nonce_bytes);

    let new_encrypted_dek = new_kek_cipher
        .encrypt(
            &Nonce::from(new_dek_nonce_bytes),
            Payload {
                msg: &dek_bytes,
                aad: tenant_id.as_bytes(),
            },
        )
        .map_err(|_| EnvelopeError::EncryptionFailed)?;

    Ok(EnvelopeEncryptedData {
        kek_id: format!("kms:tenant:{tenant_id}"),
        kek_version: new_kek_version,
        encrypted_dek: b64.encode(new_encrypted_dek),
        dek_nonce: b64.encode(new_dek_nonce_bytes),
        data_nonce: envelope.data_nonce.clone(),
        ciphertext: envelope.ciphertext.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER_KEY: &[u8; 32] = b"master_kms_secret_key_32_bytes!!";

    #[test]
    fn envelope_round_trip() {
        let tenant_id = "tenant-123";
        let plaintext = b"Confidential cloud document content";

        let envelope =
            encrypt_envelope(MASTER_KEY, tenant_id, 1, plaintext).expect("encryption succeeds");

        assert_eq!(envelope.kek_version, 1);
        assert_eq!(envelope.kek_id, "kms:tenant:tenant-123");

        let decrypted =
            decrypt_envelope(MASTER_KEY, tenant_id, &envelope).expect("decryption succeeds");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn envelope_rejects_wrong_tenant_id() {
        let plaintext = b"Sensitive tenant data";
        let envelope =
            encrypt_envelope(MASTER_KEY, "tenant-A", 1, plaintext).expect("encryption succeeds");

        let err = decrypt_envelope(MASTER_KEY, "tenant-B", &envelope);
        assert!(matches!(err, Err(EnvelopeError::DecryptionFailed)));
    }

    #[test]
    fn envelope_key_rotation() {
        let tenant_id = "tenant-xyz";
        let plaintext = b"Data requiring key rotation";

        let envelope_v1 =
            encrypt_envelope(MASTER_KEY, tenant_id, 1, plaintext).expect("v1 encryption succeeds");

        let envelope_v2 =
            rotate_envelope(MASTER_KEY, tenant_id, 2, &envelope_v1).expect("v2 rotation succeeds");

        assert_eq!(envelope_v2.kek_version, 2);
        // Ciphertext stays identical because DEK was re-wrapped without re-encrypting payload
        assert_eq!(envelope_v2.ciphertext, envelope_v1.ciphertext);

        let decrypted_v2 =
            decrypt_envelope(MASTER_KEY, tenant_id, &envelope_v2).expect("v2 decryption succeeds");

        assert_eq!(decrypted_v2, plaintext);
    }
}
