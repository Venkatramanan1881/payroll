//! Verifies Firebase phone sign-in ID tokens (D-12) and returns the verified phone number.

use crate::tokens::TokenError;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const JWK_URL: &str = "https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com";

#[async_trait::async_trait]
pub trait PhoneTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> Result<String, TokenError>;
}

struct Keys {
    map: HashMap<String, DecodingKey>,
    valid_until: Instant,
}

pub struct GoogleVerifier {
    project_id: String,
    jwk_url: &'static str,
    keys: RwLock<Keys>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct FirebaseClaims {
    sub: String,
    iat: i64,
    #[serde(default)]
    phone_number: String,
}

impl GoogleVerifier {
    /// Fetches Google's signing keys on first use and whenever their cache lifetime ends.
    pub fn new(project_id: &str) -> Self {
        Self::with_keys(project_id, HashMap::new()).expired()
    }

    /// Fixed keys (tests); they never refresh.
    pub fn with_keys(project_id: &str, map: HashMap<String, DecodingKey>) -> Self {
        let far = Instant::now() + Duration::from_secs(100 * 365 * 24 * 3600);
        Self { project_id: project_id.to_string(), jwk_url: JWK_URL, keys: RwLock::new(Keys { map, valid_until: far }) }
    }

    fn expired(self) -> Self {
        self.keys.try_write().expect("fresh lock").valid_until = Instant::now();
        self
    }

    async fn key(&self, kid: &str) -> Result<DecodingKey, TokenError> {
        {
            let k = self.keys.read().await;
            if Instant::now() < k.valid_until {
                return k.map.get(kid).cloned().ok_or(TokenError);
            }
        }
        let res = reqwest::get(self.jwk_url).await.map_err(|_| TokenError)?;
        let max_age = res
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').find_map(|p| p.trim().strip_prefix("max-age=")?.parse::<u64>().ok()))
            .unwrap_or(300);
        let set: JwkSet = res.json().await.map_err(|_| TokenError)?;
        let mut map = HashMap::new();
        for k in set.keys {
            map.insert(k.kid, DecodingKey::from_rsa_components(&k.n, &k.e).map_err(|_| TokenError)?);
        }
        let mut k = self.keys.write().await;
        *k = Keys { map, valid_until: Instant::now() + Duration::from_secs(max_age) };
        k.map.get(kid).cloned().ok_or(TokenError)
    }
}

#[async_trait::async_trait]
impl PhoneTokenVerifier for GoogleVerifier {
    async fn verify(&self, id_token: &str) -> Result<String, TokenError> {
        let header = decode_header(id_token).map_err(|_| TokenError)?;
        if header.alg != Algorithm::RS256 {
            return Err(TokenError);
        }
        let key = self.key(header.kid.as_deref().ok_or(TokenError)?).await?;
        let mut v = Validation::new(Algorithm::RS256);
        v.set_audience(&[&self.project_id]);
        v.set_issuer(&[format!("https://securetoken.google.com/{}", self.project_id)]);
        let c = decode::<FirebaseClaims>(id_token, &key, &v).map_err(|_| TokenError)?.claims;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| TokenError)?.as_secs() as i64;
        if c.sub.is_empty() || c.phone_number.is_empty() || c.iat > now + 60 {
            return Err(TokenError);
        }
        Ok(c.phone_number)
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Vendor console sign-in (D-22): Google ID tokens from the vendor's own domain.

const GOOGLE_CERTS: &str = "https://www.googleapis.com/oauth2/v3/certs";

/// Verifies a vendor's Google ID token and returns the verified email (lower case).
#[async_trait::async_trait]
pub trait VendorTokenVerifier: Send + Sync {
    async fn verify(&self, id_token: &str) -> Result<String, TokenError>;
}

pub struct GoogleIdVerifier {
    keys: GoogleVerifier,
    domain: String,
}

#[derive(Deserialize)]
struct GoogleIdClaims {
    email: String,
    #[serde(default)]
    email_verified: bool,
    #[serde(default)]
    hd: Option<String>,
    iat: i64,
}

impl GoogleIdVerifier {
    /// `client_id`: the OAuth client the console signs in with (the token audience); `domain`: the vendor's Workspace domain.
    pub fn new(client_id: &str, domain: &str) -> Self {
        let mut keys = GoogleVerifier::new(client_id);
        keys.jwk_url = GOOGLE_CERTS;
        Self { keys, domain: domain.trim().to_lowercase() }
    }

    /// Fixed keys (tests).
    pub fn with_keys(client_id: &str, domain: &str, map: HashMap<String, DecodingKey>) -> Self {
        Self { keys: GoogleVerifier::with_keys(client_id, map), domain: domain.to_lowercase() }
    }
}

#[async_trait::async_trait]
impl VendorTokenVerifier for GoogleIdVerifier {
    async fn verify(&self, id_token: &str) -> Result<String, TokenError> {
        let header = decode_header(id_token).map_err(|_| TokenError)?;
        if header.alg != Algorithm::RS256 {
            return Err(TokenError);
        }
        let key = self.keys.key(header.kid.as_deref().ok_or(TokenError)?).await?;
        let mut v = Validation::new(Algorithm::RS256);
        v.set_audience(&[&self.keys.project_id]);
        v.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
        let c = decode::<GoogleIdClaims>(id_token, &key, &v).map_err(|_| TokenError)?.claims;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| TokenError)?.as_secs() as i64;
        let email = c.email.to_lowercase();
        // `hd` is set only for Workspace accounts; a personal Gmail account with a look-alike address never passes.
        let in_domain = c.hd.as_deref().map(str::to_lowercase).as_deref() == Some(self.domain.as_str()) && email.ends_with(&format!("@{}", self.domain));
        if !c.email_verified || !in_domain || c.iat > now + 60 {
            return Err(TokenError);
        }
        Ok(email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use std::collections::HashMap;

    const PRIVATE: &str = include_str!("../tests/fixtures/firebase_test_private.pem");
    const PUBLIC: &str = include_str!("../tests/fixtures/firebase_test_public.pem");
    const PROJECT: &str = "payroll-test";

    fn now() -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
    }

    fn verifier() -> GoogleVerifier {
        let mut keys = HashMap::new();
        keys.insert("k1".to_string(), jsonwebtoken::DecodingKey::from_rsa_pem(PUBLIC.as_bytes()).unwrap());
        GoogleVerifier::with_keys(PROJECT, keys)
    }

    fn mint(kid: &str, aud: &str, iss: &str, exp: i64, phone: &str) -> String {
        let mut h = Header::new(Algorithm::RS256);
        h.kid = Some(kid.into());
        let claims = serde_json::json!({ "aud": aud, "iss": iss, "exp": exp, "iat": now() - 10, "sub": "uid1", "phone_number": phone });
        encode(&h, &claims, &EncodingKey::from_rsa_pem(PRIVATE.as_bytes()).unwrap()).unwrap()
    }

    fn iss() -> String {
        format!("https://securetoken.google.com/{PROJECT}")
    }

    #[tokio::test]
    async fn valid_token_gives_phone() {
        let t = mint("k1", PROJECT, &iss(), now() + 3600, "+919800000001");
        assert_eq!(verifier().verify(&t).await.unwrap(), "+919800000001");
    }

    #[tokio::test]
    async fn wrong_audience_rejected() {
        assert!(verifier().verify(&mint("k1", "other", &iss(), now() + 3600, "+91")).await.is_err());
    }

    #[tokio::test]
    async fn wrong_issuer_rejected() {
        assert!(verifier().verify(&mint("k1", PROJECT, "https://evil", now() + 3600, "+91")).await.is_err());
    }

    #[tokio::test]
    async fn expired_rejected() {
        assert!(verifier().verify(&mint("k1", PROJECT, &iss(), now() - 3600, "+91")).await.is_err());
    }

    #[tokio::test]
    async fn unknown_kid_rejected() {
        assert!(verifier().verify(&mint("k9", PROJECT, &iss(), now() + 3600, "+91")).await.is_err());
    }

    fn google(aud: &str, iss: &str, email: &str, verified: bool, hd: Option<&str>) -> String {
        let mut h = Header::new(Algorithm::RS256);
        h.kid = Some("k1".into());
        let claims = serde_json::json!({ "aud": aud, "iss": iss, "exp": now() + 3600, "iat": now() - 10, "sub": "g1", "email": email,
                                          "email_verified": verified, "hd": hd });
        encode(&h, &claims, &EncodingKey::from_rsa_pem(PRIVATE.as_bytes()).unwrap()).unwrap()
    }

    #[tokio::test]
    async fn vendor_google_tokens_need_the_domain() {
        let mut keys = HashMap::new();
        keys.insert("k1".to_string(), jsonwebtoken::DecodingKey::from_rsa_pem(PUBLIC.as_bytes()).unwrap());
        let v = GoogleIdVerifier::with_keys("client-1", "probeyond.in", keys);
        let iss = "https://accounts.google.com";
        assert_eq!(v.verify(&google("client-1", iss, "Ops@ProBeyond.in", true, Some("probeyond.in"))).await.unwrap(), "ops@probeyond.in");
        assert!(v.verify(&google("client-1", iss, "ops@probeyond.in", true, None)).await.is_err(), "personal account without hd");
        assert!(v.verify(&google("client-1", iss, "ops@gmail.com", true, Some("probeyond.in"))).await.is_err());
        assert!(v.verify(&google("client-1", iss, "ops@probeyond.in", false, Some("probeyond.in"))).await.is_err(), "unverified email");
        assert!(v.verify(&google("other", iss, "ops@probeyond.in", true, Some("probeyond.in"))).await.is_err(), "wrong audience");
        assert!(v.verify(&google("client-1", "https://evil", "ops@probeyond.in", true, Some("probeyond.in"))).await.is_err());
    }

    #[tokio::test]
    async fn missing_phone_rejected() {
        assert!(verifier().verify(&mint("k1", PROJECT, &iss(), now() + 3600, "")).await.is_err());
    }
}
