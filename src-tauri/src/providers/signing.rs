//! Only the fixed read/handshake manifest can reach signing.
use super::{Failure, Route};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::{Signer, pkcs8::DecodePrivateKey};
use rsa::{
    RsaPrivateKey,
    pkcs1::DecodeRsaPrivateKey,
    signature::{RandomizedSigner, SignatureEncoding},
    traits::PublicKeyParts,
};
use sha2::Sha256;

#[derive(Clone)]
pub enum Key {
    Ed25519(Box<ed25519_dalek::SigningKey>),
    Rsa(Box<rsa::pss::SigningKey<Sha256>>),
}
impl Key {
    pub fn parse(pem: &str) -> Result<Self, Failure> {
        if pem.is_empty() || pem.len() > 16384 {
            return Err(Failure::InvalidCredential);
        }
        let pem = pem.trim();
        if let Ok(key) = ed25519_dalek::SigningKey::from_pkcs8_pem(pem) {
            return Ok(Self::Ed25519(Box::new(key)));
        }
        let key = RsaPrivateKey::from_pkcs8_pem(pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(pem))
            .map_err(|_| Failure::InvalidCredential)?;
        if key.n().bits() != 2048 {
            return Err(Failure::InvalidCredential);
        }
        key.validate().map_err(|_| Failure::InvalidCredential)?;
        Ok(Self::Rsa(Box::new(rsa::pss::SigningKey::new(key))))
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Ed25519(_) => "ed25519",
            Self::Rsa(_) => "rsa",
        }
    }
    pub fn sign(&self, route: &Route, timestamp_ms: u64) -> Result<String, Failure> {
        let bytes = presign(route, timestamp_ms)?;
        Ok(STANDARD.encode(match self {
            Self::Ed25519(key) => key.sign(bytes.as_bytes()).to_bytes().to_vec(),
            Self::Rsa(key) => key
                .try_sign_with_rng(&mut rsa::rand_core::OsRng, bytes.as_bytes())
                .map_err(|_| Failure::Signing)?
                .to_vec(),
        }))
    }
}
pub fn presign(route: &Route, timestamp_ms: u64) -> Result<String, Failure> {
    let path = route.path()?;
    Ok(format!(
        "{timestamp_ms}GET{}",
        path.split('?').next().ok_or(Failure::InvalidRequest)?
    ))
}
