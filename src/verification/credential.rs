use chrono::{DateTime, Utc};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::types::error::PanopticonError;

/// A verifiable credential issued by one agent about another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    pub issuer_id: Uuid,
    pub subject_id: Uuid,
    pub claims: HashMap<String, String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub signature: Vec<u8>,
}

impl VerifiableCredential {
    /// Create the canonical bytes to be signed for this credential.
    fn signing_payload(
        issuer_id: &Uuid,
        subject_id: &Uuid,
        claims: &HashMap<String, String>,
        issued_at: &DateTime<Utc>,
        expires_at: &Option<DateTime<Utc>>,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(issuer_id.as_bytes());
        payload.extend_from_slice(subject_id.as_bytes());
        // Sort keys for deterministic serialization
        let mut sorted_claims: Vec<_> = claims.iter().collect();
        sorted_claims.sort_by_key(|(k, _)| (*k).clone());
        for (k, v) in &sorted_claims {
            payload.extend_from_slice(k.as_bytes());
            payload.extend_from_slice(v.as_bytes());
        }
        payload.extend_from_slice(issued_at.to_rfc3339().as_bytes());
        if let Some(exp) = expires_at {
            payload.extend_from_slice(exp.to_rfc3339().as_bytes());
        }
        payload
    }

    /// Issue a new credential signed with the issuer's signing key.
    pub fn issue(
        issuer_id: Uuid,
        subject_id: Uuid,
        claims: HashMap<String, String>,
        expires_at: Option<DateTime<Utc>>,
        signing_key: &SigningKey,
    ) -> Self {
        let issued_at = Utc::now();
        let payload =
            Self::signing_payload(&issuer_id, &subject_id, &claims, &issued_at, &expires_at);
        let signature = signing_key.sign(&payload);

        Self {
            issuer_id,
            subject_id,
            claims,
            issued_at,
            expires_at,
            signature: signature.to_bytes().to_vec(),
        }
    }

    /// Verify this credential's signature against the issuer's public key.
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<(), PanopticonError> {
        let payload = Self::signing_payload(
            &self.issuer_id,
            &self.subject_id,
            &self.claims,
            &self.issued_at,
            &self.expires_at,
        );
        let sig_bytes: [u8; 64] =
            self.signature.as_slice().try_into().map_err(|_| {
                PanopticonError::VerificationFailed("Invalid signature length".into())
            })?;
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(&payload, &signature)
            .map_err(|e| PanopticonError::VerificationFailed(format!("Signature invalid: {e}")))
    }
}

/// Verify a delegation chain of credentials A->B->C.
/// Each credential[i] must have its signature verified with the corresponding public key.
/// Additionally, the chain must be contiguous: credential[i].subject_id == credential[i+1].issuer_id.
pub fn verify_credential_chain(
    credentials: &[VerifiableCredential],
    public_keys: &[VerifyingKey],
) -> Result<(), PanopticonError> {
    if credentials.len() != public_keys.len() {
        return Err(PanopticonError::VerificationFailed(
            "Number of credentials and public keys must match".into(),
        ));
    }
    if credentials.is_empty() {
        return Ok(());
    }

    // Verify each signature
    for (i, (cred, key)) in credentials.iter().zip(public_keys.iter()).enumerate() {
        cred.verify_signature(key).map_err(|e| {
            PanopticonError::VerificationFailed(format!(
                "Credential {i} signature verification failed: {e}"
            ))
        })?;
    }

    // Verify chain continuity
    for i in 0..credentials.len() - 1 {
        if credentials[i].subject_id != credentials[i + 1].issuer_id {
            return Err(PanopticonError::VerificationFailed(format!(
                "Chain break at index {i}: subject {} != next issuer {}",
                credentials[i].subject_id,
                credentials[i + 1].issuer_id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    fn gen_keypair() -> (SigningKey, VerifyingKey) {
        let mut secret = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        let signing = SigningKey::from_bytes(&secret);
        let verifying = signing.verifying_key();
        (signing, verifying)
    }

    #[test]
    fn test_issue_and_verify_credential() {
        let (signing_key, verifying_key) = gen_keypair();
        let issuer = Uuid::new_v4();
        let subject = Uuid::new_v4();

        let mut claims = HashMap::new();
        claims.insert("role".to_string(), "auditor".to_string());

        let cred = VerifiableCredential::issue(issuer, subject, claims, None, &signing_key);

        assert_eq!(cred.issuer_id, issuer);
        assert_eq!(cred.subject_id, subject);
        assert!(cred.verify_signature(&verifying_key).is_ok());
    }

    #[test]
    fn test_tampered_credential_fails() {
        let (signing_key, verifying_key) = gen_keypair();
        let issuer = Uuid::new_v4();
        let subject = Uuid::new_v4();

        let mut claims = HashMap::new();
        claims.insert("role".to_string(), "auditor".to_string());

        let mut cred = VerifiableCredential::issue(issuer, subject, claims, None, &signing_key);
        // Tamper with the claims
        cred.claims.insert("role".to_string(), "admin".to_string());

        assert!(cred.verify_signature(&verifying_key).is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let (signing_key, _) = gen_keypair();
        let (_, wrong_verifying_key) = gen_keypair();
        let issuer = Uuid::new_v4();
        let subject = Uuid::new_v4();

        let cred = VerifiableCredential::issue(issuer, subject, HashMap::new(), None, &signing_key);

        assert!(cred.verify_signature(&wrong_verifying_key).is_err());
    }

    #[test]
    fn test_credential_chain_verification() {
        let (key_a, pub_a) = gen_keypair();
        let (key_b, pub_b) = gen_keypair();

        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();
        let agent_c = Uuid::new_v4();

        let mut claims_ab = HashMap::new();
        claims_ab.insert("delegation".to_string(), "task-x".to_string());
        let cred_ab = VerifiableCredential::issue(agent_a, agent_b, claims_ab, None, &key_a);

        let mut claims_bc = HashMap::new();
        claims_bc.insert("delegation".to_string(), "task-x-sub".to_string());
        let cred_bc = VerifiableCredential::issue(agent_b, agent_c, claims_bc, None, &key_b);

        let result = verify_credential_chain(&[cred_ab, cred_bc], &[pub_a, pub_b]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_credential_chain_broken() {
        let (key_a, pub_a) = gen_keypair();
        let (key_b, pub_b) = gen_keypair();

        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();
        let agent_c = Uuid::new_v4();
        let agent_d = Uuid::new_v4();

        // A -> B
        let cred_ab = VerifiableCredential::issue(agent_a, agent_b, HashMap::new(), None, &key_a);
        // C -> D (not B -> C, so chain is broken)
        let cred_cd = VerifiableCredential::issue(agent_c, agent_d, HashMap::new(), None, &key_b);

        let result = verify_credential_chain(&[cred_ab, cred_cd], &[pub_a, pub_b]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_chain_ok() {
        let result = verify_credential_chain(&[], &[]);
        assert!(result.is_ok());
    }
}
