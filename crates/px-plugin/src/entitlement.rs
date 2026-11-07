//! Offline entitlement verification; no token issuer or billing service.
use ed25519_dalek::{Signature, VerifyingKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Claims {
    pub plugin_id: String,
    pub tier: String,
    pub major: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meters: Option<BTreeMap<String, u64>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedToken {
    pub claims: Claims,
    pub signature: String,
}
pub trait EntitlementVerifier {
    fn verify(&self, plugin_id: &str, major: u64, now: i64) -> Result<(), String>;
}
pub struct Free;
impl EntitlementVerifier for Free {
    fn verify(&self, _: &str, _: u64, _: i64) -> Result<(), String> {
        Ok(())
    }
}
pub struct TokenVerifier<'a> {
    pub token: &'a SignedToken,
    pub key: &'a VerifyingKey,
}
impl EntitlementVerifier for TokenVerifier<'_> {
    fn verify(&self, plugin_id: &str, major: u64, now: i64) -> Result<(), String> {
        let claims = &self.token.claims;
        if claims.plugin_id != plugin_id || claims.major != major || claims.tier.is_empty() {
            return Err("token claims do not match plugin".into());
        }
        if claims.exp.is_some_and(|exp| now >= exp) {
            return Err("entitlement expired".into());
        }
        let bytes = hex::decode(&self.token.signature).map_err(|_| "invalid signature encoding")?;
        let signature = Signature::from_slice(&bytes).map_err(|_| "invalid signature")?;
        let message = serde_json::to_vec(claims).map_err(|e| e.to_string())?;
        self.key
            .verify_strict(&message, &signature)
            .map_err(|_| "invalid signature".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    #[test]
    fn signed_tokens_enforce_identity_expiry_and_signature() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let claims = Claims {
            plugin_id: "px.example".into(),
            tier: "paid".into(),
            major: 1,
            exp: Some(100),
            meters: None,
        };
        let signature = hex::encode(key.sign(&serde_json::to_vec(&claims).unwrap()).to_bytes());
        let mut token = SignedToken { claims, signature };
        assert!(
            TokenVerifier {
                token: &token,
                key: &key.verifying_key()
            }
            .verify("px.example", 1, 99)
            .is_ok()
        );
        assert!(
            TokenVerifier {
                token: &token,
                key: &key.verifying_key()
            }
            .verify("px.example", 1, 100)
            .is_err()
        );
        assert!(
            TokenVerifier {
                token: &token,
                key: &key.verifying_key()
            }
            .verify("other", 1, 0)
            .is_err()
        );
        assert!(
            TokenVerifier {
                token: &token,
                key: &SigningKey::from_bytes(&[8; 32]).verifying_key()
            }
            .verify("px.example", 1, 0)
            .is_err()
        );
        token.claims.tier = "modified".into();
        assert!(
            TokenVerifier {
                token: &token,
                key: &key.verifying_key()
            }
            .verify("px.example", 1, 0)
            .is_err()
        );
    }
}
