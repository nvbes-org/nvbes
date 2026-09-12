use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use rand::RngCore;
use uuid::Uuid;

pub struct SealedSecret {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    pub key_version: i16,
}

pub struct MfaCrypto {
    active_version: i16,
    active: Aes256Gcm,
    previous: Option<(i16, Aes256Gcm)>,
}

impl MfaCrypto {
    pub fn with_rotation(
        active_version: i16,
        active_key: [u8; 32],
        previous: Option<(i16, [u8; 32])>,
    ) -> anyhow::Result<Self> {
        if active_version <= 0
            || previous.is_some_and(|(version, _)| version <= 0 || version == active_version)
        {
            anyhow::bail!("MFA key versions are invalid");
        }
        Ok(Self {
            active_version,
            active: Aes256Gcm::new(&active_key.into()),
            previous: previous.map(|(version, key)| (version, Aes256Gcm::new(&key.into()))),
        })
    }

    pub fn active_version(&self) -> i16 {
        self.active_version
    }

    pub fn seal(&self, factor_id: Uuid, secret: &str) -> anyhow::Result<SealedSecret> {
        let mut nonce_bytes = [0_u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);
        let ciphertext = self
            .active
            .encrypt(
                &nonce,
                Payload {
                    msg: secret.as_bytes(),
                    aad: &associated_data(factor_id),
                },
            )
            .map_err(|_| anyhow::anyhow!("MFA secret encryption failed"))?;
        Ok(SealedSecret {
            ciphertext,
            nonce: nonce.into(),
            key_version: self.active_version,
        })
    }

    pub fn open(
        &self,
        factor_id: Uuid,
        key_version: i16,
        ciphertext: &[u8],
        nonce: &[u8],
    ) -> anyhow::Result<String> {
        let nonce: [u8; 12] = nonce
            .try_into()
            .map_err(|_| anyhow::anyhow!("MFA secret nonce is invalid"))?;
        let cipher = if key_version == self.active_version {
            &self.active
        } else if let Some((previous_version, cipher)) = &self.previous
            && key_version == *previous_version
        {
            cipher
        } else {
            anyhow::bail!("MFA secret key version is unavailable");
        };
        let plaintext = cipher
            .decrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: ciphertext,
                    aad: &associated_data(factor_id),
                },
            )
            .map_err(|_| anyhow::anyhow!("MFA secret decryption failed"))?;
        String::from_utf8(plaintext).map_err(|_| anyhow::anyhow!("MFA secret encoding is invalid"))
    }
}

fn associated_data(factor_id: Uuid) -> Vec<u8> {
    format!("nvbes-identity:v1:{factor_id}:totp-secret").into_bytes()
}

#[cfg(test)]
#[path = "identity.mfa.crypto.tests.rs"]
mod tests;
