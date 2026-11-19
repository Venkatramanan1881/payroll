//! Access tokens (HS256, 900 s) and refresh tokens "<org_id>.<random>" stored only as SHA-256 hashes (TSD §8).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const ACCESS_TTL_SECS: i64 = 900;

/// Unknown fields are refused, so no other token type (choice, second-step challenge, vendor) can pass as an access token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AccessClaims {
    pub sub: Uuid,
    pub org: Uuid,
    /// Session that issued the token; sign-out revokes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sid: Option<Uuid>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug)]
pub struct TokenError;

pub fn issue_access(secret: &[u8], user_id: Uuid, org_id: Uuid, sid: Option<Uuid>, now: i64) -> String {
    let claims = AccessClaims { sub: user_id, org: org_id, sid, iat: now, exp: now + ACCESS_TTL_SECS };
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


/// Short-lived (300 s) proof that a phone number was verified, listing the (organisation, user) pairs it may pick (OQ-C03-05).
#[derive(Debug, Serialize, Deserialize)]
struct ChoiceClaims {
    cands: Vec<(Uuid, Uuid)>,
    /// Set for a password sign-in: the chosen organisation's AC-08 second step still applies.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pw: bool,
    exp: i64,
}

pub fn issue_choice(secret: &[u8], cands: Vec<(Uuid, Uuid)>, now: i64) -> String {
    encode(&Header::new(Algorithm::HS256), &ChoiceClaims { cands, pw: false, exp: now + 300 }, &EncodingKey::from_secret(secret))
        .expect("HS256 encoding cannot fail")
}

/// A choice after a password sign-in: the chosen organisation's second step is still owed.
pub fn issue_password_choice(secret: &[u8], cands: Vec<(Uuid, Uuid)>, now: i64) -> String {
    encode(&Header::new(Algorithm::HS256), &ChoiceClaims { cands, pw: true, exp: now + 300 }, &EncodingKey::from_secret(secret))
        .expect("HS256 encoding cannot fail")
}

/// (candidates, whether the second step is still owed).
pub fn verify_choice(secret: &[u8], token: &str, now: i64) -> Result<(Vec<(Uuid, Uuid)>, bool), TokenError> {
    let mut v = Validation::new(Algorithm::HS256);
    v.validate_exp = false;
    v.required_spec_claims.clear();
    let c = decode::<ChoiceClaims>(token, &DecodingKey::from_secret(secret), &v).map_err(|_| TokenError)?.claims;
    if now > c.exp {
        return Err(TokenError);
    }
    Ok((c.cands, c.pw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    const SECRET: &[u8] = b"secret";

    #[test]
    fn access_roundtrip() {
        let (u, o) = (Uuid::new_v4(), Uuid::new_v4());
        let sid = Some(Uuid::new_v4());
        let t = issue_access(SECRET, u, o, sid, 1_000);
        let c = verify_access(SECRET, &t, 1_000).unwrap();
        assert_eq!((c.sub, c.org, c.sid, c.exp), (u, o, sid, 1_000 + ACCESS_TTL_SECS));
    }

    #[test]
    fn access_expired_rejected() {
        let t = issue_access(SECRET, Uuid::new_v4(), Uuid::new_v4(), None, 1_000);
        assert!(verify_access(SECRET, &t, 1_000 + ACCESS_TTL_SECS).is_ok());
        assert!(verify_access(SECRET, &t, 1_000 + ACCESS_TTL_SECS + 1).is_err());
    }

    #[test]
    fn access_wrong_secret_rejected() {
        let t = issue_access(SECRET, Uuid::new_v4(), Uuid::new_v4(), None, 1_000);
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

// ---------------------------------------------------------------------------------------------------------------------
// Second-step challenge (AC-08) and vendor tokens (D-22). Each carries `typ` and refuses unknown fields.

fn hs256() -> Validation {
    let mut v = Validation::new(Algorithm::HS256);
    v.validate_exp = false; // checked against `now`
    v.required_spec_claims.clear();
    v
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChallengeClaims {
    typ: String,
    cu: Uuid,
    co: Uuid,
    /// A new authenticator secret awaiting its first code, encrypted with the MFA key (base64url).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pend: Option<String>,
    exp: i64,
}

pub const CHALLENGE_TTL_SECS: i64 = 300;

pub fn issue_challenge(secret: &[u8], user: Uuid, org: Uuid, pending: Option<String>, now: i64) -> String {
    let c = ChallengeClaims { typ: "mfa".into(), cu: user, co: org, pend: pending, exp: now + CHALLENGE_TTL_SECS };
    encode(&Header::new(Algorithm::HS256), &c, &EncodingKey::from_secret(secret)).expect("HS256 encoding cannot fail")
}

/// (user, organisation, pending encrypted secret).
pub fn verify_challenge(secret: &[u8], token: &str, now: i64) -> Result<(Uuid, Uuid, Option<String>), TokenError> {
    let c = decode::<ChallengeClaims>(token, &DecodingKey::from_secret(secret), &hs256()).map_err(|_| TokenError)?.claims;
    if c.typ != "mfa" || now > c.exp {
        return Err(TokenError);
    }
    Ok((c.cu, c.co, c.pend))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VendorClaims {
    typ: String,
    vid: Uuid,
    iat: i64,
    exp: i64,
}

pub const VENDOR_TTL_SECS: i64 = 3600;

pub fn issue_vendor(secret: &[u8], vendor_user: Uuid, now: i64) -> String {
    let c = VendorClaims { typ: "vendor".into(), vid: vendor_user, iat: now, exp: now + VENDOR_TTL_SECS };
    encode(&Header::new(Algorithm::HS256), &c, &EncodingKey::from_secret(secret)).expect("HS256 encoding cannot fail")
}

pub fn verify_vendor(secret: &[u8], token: &str, now: i64) -> Result<Uuid, TokenError> {
    let c = decode::<VendorClaims>(token, &DecodingKey::from_secret(secret), &hs256()).map_err(|_| TokenError)?.claims;
    if c.typ != "vendor" || now > c.exp {
        return Err(TokenError);
    }
    Ok(c.vid)
}

#[cfg(test)]
mod kind_tests {
    use super::*;

    const S: &[u8] = b"k";

    #[test]
    fn token_kinds_never_cross() {
        let (u, o) = (Uuid::new_v4(), Uuid::new_v4());
        let access = issue_access(S, u, o, None, 1_000);
        let challenge = issue_challenge(S, u, o, Some("x".into()), 1_000);
        let vendor = issue_vendor(S, u, 1_000);
        let choice = issue_choice(S, vec![(o, u)], 1_000);
        for t in [&challenge, &vendor, &choice] {
            assert!(verify_access(S, t, 1_001).is_err(), "only access tokens are access tokens");
        }
        for t in [&access, &vendor, &choice] {
            assert!(verify_challenge(S, t, 1_001).is_err());
        }
        for t in [&access, &challenge, &choice] {
            assert!(verify_vendor(S, t, 1_001).is_err());
        }
        assert_eq!(verify_challenge(S, &challenge, 1_001).unwrap(), (u, o, Some("x".into())));
        assert!(verify_challenge(S, &challenge, 1_000 + CHALLENGE_TTL_SECS + 1).is_err());
        assert_eq!(verify_vendor(S, &vendor, 1_001).unwrap(), u);
    }
}
