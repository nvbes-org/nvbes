use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Debug)]
pub struct SealedValue {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
}

#[derive(Clone)]
pub struct EmailCrypto {
    cipher: Aes256Gcm,
    recipient_hmac_key: [u8; 32],
}

impl EmailCrypto {
    pub fn new(data_key: [u8; 32], recipient_hmac_key: [u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new(&data_key.into()),
            recipient_hmac_key,
        }
    }

    pub fn seal(
        &self,
        message_id: Uuid,
        field: &'static str,
        plaintext: &[u8],
    ) -> anyhow::Result<SealedValue> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad: &associated_data(message_id, field),
                },
            )
            .map_err(|_| anyhow::anyhow!("email data encryption failed"))?;
        Ok(SealedValue {
            ciphertext,
            nonce: nonce.into(),
        })
    }

    pub fn open(
        &self,
        message_id: Uuid,
        field: &'static str,
        sealed: &SealedValue,
    ) -> anyhow::Result<Vec<u8>> {
        self.cipher
            .decrypt(
                Nonce::from_slice(&sealed.nonce),
                Payload {
                    msg: &sealed.ciphertext,
                    aad: &associated_data(message_id, field),
                },
            )
            .map_err(|_| anyhow::anyhow!("email data decryption failed"))
    }

    pub fn recipient_hash(&self, email: &str) -> [u8; 32] {
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.recipient_hmac_key)
            .expect("HMAC accepts 32-byte keys");
        mac.update(email.trim().to_ascii_lowercase().as_bytes());
        mac.finalize().into_bytes().into()
    }
}

fn associated_data(message_id: Uuid, field: &'static str) -> Vec<u8> {
    format!("nvbes-email:v1:{message_id}:{field}").into_bytes()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{EmailCrypto, SealedValue};

    #[test]
    fn envelope_data_is_bound_to_message_and_field() {
        let crypto = EmailCrypto::new([7; 32], [9; 32]);
        let message_id = Uuid::new_v4();
        let sealed = crypto
            .seal(message_id, "recipient", b"person@example.com")
            .unwrap();

        assert_eq!(
            crypto.open(message_id, "recipient", &sealed).unwrap(),
            b"person@example.com"
        );
        assert!(crypto.open(Uuid::new_v4(), "recipient", &sealed).is_err());
        assert!(crypto.open(message_id, "template", &sealed).is_err());

        let tampered = SealedValue {
            ciphertext: vec![0; sealed.ciphertext.len()],
            nonce: sealed.nonce,
        };
        assert!(crypto.open(message_id, "recipient", &tampered).is_err());
    }

    #[test]
    fn recipient_hash_is_normalized_and_keyed() {
        let crypto = EmailCrypto::new([7; 32], [9; 32]);
        assert_eq!(
            crypto.recipient_hash(" Person@Example.com "),
            crypto.recipient_hash("person@example.com")
        );
        assert_ne!(
            crypto.recipient_hash("person@example.com"),
            EmailCrypto::new([7; 32], [8; 32]).recipient_hash("person@example.com")
        );
    }
}
