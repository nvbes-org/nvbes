use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum PinnedKeyError {
    #[error("invalid pinned Identity verification key configuration")]
    Configuration,
    #[error("unknown or retired Identity signing key")]
    Untrusted,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlapKey {
    kid: String,
    public_key_pem: String,
    accept_until: u64,
}

/// Operator-pinned keys only. Token headers can select a configured key, never a URL.
#[derive(Clone)]
pub struct PinnedKeySet {
    keys: BTreeMap<String, (DecodingKey, Option<u64>)>,
}

impl PinnedKeySet {
    pub fn new(
        kid: &str,
        public_key_pem: &str,
        overlap_json: &str,
    ) -> Result<Self, PinnedKeyError> {
        if overlap_json.len() > 65_536 {
            return Err(PinnedKeyError::Configuration);
        }
        let overlap: Vec<OverlapKey> =
            serde_json::from_str(overlap_json).map_err(|_| PinnedKeyError::Configuration)?;
        if overlap.len() > 3 {
            return Err(PinnedKeyError::Configuration);
        }
        let mut keys = BTreeMap::from([(kid.to_owned(), (decode_key(kid, public_key_pem)?, None))]);
        for entry in overlap {
            if entry.accept_until == 0 || keys.contains_key(&entry.kid) {
                return Err(PinnedKeyError::Configuration);
            }
            let key = decode_key(&entry.kid, &entry.public_key_pem)?;
            keys.insert(entry.kid, (key, Some(entry.accept_until)));
        }
        Ok(Self { keys })
    }

    /// No grace period on retired keys, even if an otherwise valid JWT has time remaining.
    pub fn key(&self, kid: Option<&str>, now: u64) -> Result<&DecodingKey, PinnedKeyError> {
        let (key, until) = kid
            .and_then(|id| self.keys.get(id))
            .ok_or(PinnedKeyError::Untrusted)?;
        if until.is_some_and(|until| now >= until) {
            return Err(PinnedKeyError::Untrusted);
        }
        Ok(key)
    }
}

fn decode_key(kid: &str, pem: &str) -> Result<DecodingKey, PinnedKeyError> {
    if kid.is_empty()
        || kid.len() > 128
        || !kid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
        || pem.len() > 16_384
    {
        return Err(PinnedKeyError::Configuration);
    }
    DecodingKey::from_rsa_pem(pem.as_bytes()).map_err(|_| PinnedKeyError::Configuration)
}
