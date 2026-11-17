//! Access tokens (HS256, 900 s) and refresh tokens "<org_id>.<random>" stored only as SHA-256 hashes (TSD §8).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const ACCESS_TTL_SECS: i64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub org: Uuid,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug)]
pub struct TokenError;

pub fn issue_access(secret: &[u8], user_id: Uuid, org_id: Uuid, now: i64) -> String {
    let claims = AccessClaims { sub: user_id, org: org_id, iat: now, exp: now + ACCESS_TTL_SECS };
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret)).expect("HS256 encoding cannot fail")
}

pub fn verify_access(secret: &[u8], token: &str, now: i64) -> Result<AccessClaims, TokenError> {
    let mut v = Validation::new(Algorithm::HS256);
    v.validate_exp = false; // checked below against `now`, so expiry is deterministic
    v.required_spec_claims.clear();
    let claims = decode::<AccessClaims>(token, &DecodingKey::from_secret(secret), &v).map_err(|_| TokenError)?.claims;
    if now > claims.exp {
        return Err(TokenError);
    }
    Ok(claims)
}

pub fn hash_refresh(secret_part: &str) -> [u8; 32] {
    Sha256::digest(secret_part.as_bytes()).into()
}

pub fn new_refresh(org_id: Uuid) -> (String, [u8; 32]) {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let secret_part = URL_SAFE_NO_PAD.encode(raw);
    let hash = hash_refresh(&secret_part);
    (format!("{org_id}.{secret_part}"), hash)
}

pub fn parse_refresh(token: &str) -> Option<(Uuid, [u8; 32])> {
    let (org, secret_part) = token.split_once('.')?;
    let org = Uuid::parse_str(org).ok()?;
    if URL_SAFE_NO_PAD.decode(secret_part).ok()?.len() != 32 {
        return None;
    }
    Some((org, hash_refresh(secret_part)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    const SECRET: &[u8] = b"secret";

    #[test]
    fn access_roundtrip() {
        let (u, o) = (Uuid::new_v4(), Uuid::new_v4());
        let t = issue_access(SECRET, u, o, 1_000);
        let c = verify_access(SECRET, &t, 1_000).unwrap();
        assert_eq!((c.sub, c.org, c.exp), (u, o, 1_000 + ACCESS_TTL_SECS));
    }

    #[test]
    fn access_expired_rejected() {
        let t = issue_access(SECRET, Uuid::new_v4(), Uuid::new_v4(), 1_000);
        assert!(verify_access(SECRET, &t, 1_000 + ACCESS_TTL_SECS).is_ok());
        assert!(verify_access(SECRET, &t, 1_000 + ACCESS_TTL_SECS + 1).is_err());
    }

    #[test]
    fn access_wrong_secret_rejected() {
        let t = issue_access(SECRET, Uuid::new_v4(), Uuid::new_v4(), 1_000);
        assert!(verify_access(b"other", &t, 1_000).is_err());
    }

    #[test]
    fn refresh_roundtrip_and_garbage() {
        let org = Uuid::new_v4();
        let (token, hash) = new_refresh(org);
        assert_eq!(parse_refresh(&token), Some((org, hash)));
        for bad in ["", "abc", "not-a-uuid.xxxx", &format!("{org}."), &format!("{org}.!!!")] {
            assert_eq!(parse_refresh(bad), None, "{bad}");
        }
    }
}
