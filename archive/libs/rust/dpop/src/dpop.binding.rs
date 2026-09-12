use serde::{Deserialize, Serialize};

use crate::keys::{Jwk, jwk_thumbprint};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationClaim {
    pub jkt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfirmation {
    pub cnf: ConfirmationClaim,
}

impl TokenConfirmation {
    pub fn new(jwk: &Jwk) -> Self {
        Self {
            cnf: ConfirmationClaim {
                jkt: jwk_thumbprint(jwk),
            },
        }
    }

    pub fn from_jkt(jkt: &str) -> Self {
        Self {
            cnf: ConfirmationClaim {
                jkt: jkt.to_string(),
            },
        }
    }

    pub fn verify(&self, jwk: &Jwk) -> bool {
        jwk_thumbprint(jwk) == self.cnf.jkt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::generate_key_pair;

    #[test]
    fn token_confirmation_matches_same_key() {
        let pair = generate_key_pair();
        let binding = TokenConfirmation::new(&pair.jwk);
        assert!(binding.verify(&pair.jwk));
    }

    #[test]
    fn token_confirmation_rejects_different_key() {
        let pair1 = generate_key_pair();
        let pair2 = generate_key_pair();
        let binding = TokenConfirmation::new(&pair1.jwk);
        assert!(!binding.verify(&pair2.jwk));
    }
}
