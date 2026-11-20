//! Object storage for payroll input exports (D-19: Google Cloud Storage, private bucket, short-lived signed links,
//! TB-02). On Cloud Run the service account comes from the metadata server, so no key file exists anywhere: uploads
//! use its access token and V4 signed URLs are signed through the IAM Credentials `signBlob` API.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[async_trait::async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, path: &str, body: Vec<u8>, content_type: &str) -> Result<()>;
    /// A GET link valid for `ttl_secs`.
    async fn signed_url(&self, path: &str, ttl_secs: u64) -> Result<String>;
    /// A PUT link valid for `ttl_secs` that uploads one object at `path` (TB-02).
    async fn signed_put(&self, path: &str, ttl_secs: u64) -> Result<String>;
}

const METADATA: &str = "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default";

pub struct Gcs {
    bucket: String,
    http: reqwest::Client,
}

impl Gcs {
    pub fn new(bucket: String) -> Self {
        // A hung Google call must not hold a request and its database transaction open.
        let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().expect("HTTP client");
        Gcs { bucket, http }
    }

    async fn metadata(&self, what: &str) -> Result<String> {
        let r = self.http.get(format!("{METADATA}/{what}")).header("Metadata-Flavor", "Google").send().await?.error_for_status()?;
        Ok(r.text().await?)
    }

    // shortcut: a token per call; cache it until expiry when exports become frequent.
    async fn token(&self) -> Result<String> {
        let v: Value = serde_json::from_str(&self.metadata("token").await?)?;
        v["access_token"].as_str().map(str::to_string).ok_or_else(|| anyhow!("no access_token from the metadata server"))
    }
}

#[async_trait::async_trait]
impl ObjectStore for Gcs {
    async fn put(&self, path: &str, body: Vec<u8>, content_type: &str) -> Result<()> {
        let url = format!("https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}", self.bucket, encode(path, false));
        self.http
            .post(url)
            .bearer_auth(self.token().await?)
            .header("content-type", content_type)
            .body(body)
            .send()
            .await?
            .error_for_status()
            .context("upload to Cloud Storage")?;
        Ok(())
    }

    async fn signed_url(&self, path: &str, ttl_secs: u64) -> Result<String> {
        self.sign("GET", path, ttl_secs).await
    }

    async fn signed_put(&self, path: &str, ttl_secs: u64) -> Result<String> {
        self.sign("PUT", path, ttl_secs).await
    }
}

impl Gcs {
    async fn sign(&self, method: &str, path: &str, ttl_secs: u64) -> Result<String> {
        let email = self.metadata("email").await?;
        let (url, to_sign) = v4_request(method, &self.bucket, path, &email, Utc::now(), ttl_secs);
        let b64 = base64::engine::general_purpose::STANDARD;
        let v: Value = self
            .http
            .post(format!("https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/{email}:signBlob"))
            .bearer_auth(self.token().await?)
            .json(&json!({ "payload": b64.encode(to_sign) }))
            .send()
            .await?
            .error_for_status()
            .context("signBlob (the service account needs roles/iam.serviceAccountTokenCreator on itself)")?
            .json()
            .await?;
        let sig = b64.decode(v["signedBlob"].as_str().ok_or_else(|| anyhow!("no signedBlob"))?)?;
        Ok(format!("{url}&X-Goog-Signature={}", hex(&sig)))
    }
}

/// RFC 3986 percent-encoding; `/` kept when `keep_slash` (object paths).
fn encode(s: &str, keep_slash: bool) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            b'/' if keep_slash => "/".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The unsigned V4 URL (without the signature) and the string to sign, per Google's V4 signing process.
fn v4_request(method: &str, bucket: &str, path: &str, email: &str, now: DateTime<Utc>, ttl_secs: u64) -> (String, String) {
    let stamp = now.format("%Y%m%dT%H%M%SZ").to_string();
    let scope = format!("{}/auto/storage/goog4_request", now.format("%Y%m%d"));
    let resource = format!("/{bucket}/{}", encode(path, true));
    let query = [
        ("X-Goog-Algorithm", "GOOG4-RSA-SHA256".to_string()),
        ("X-Goog-Credential", format!("{email}/{scope}")),
        ("X-Goog-Date", stamp.clone()),
        ("X-Goog-Expires", ttl_secs.to_string()),
        ("X-Goog-SignedHeaders", "host".to_string()),
    ]
    .iter()
    .map(|(k, v)| format!("{k}={}", encode(v, false)))
    .collect::<Vec<_>>()
    .join("&");
    let canonical = format!("{method}\n{resource}\n{query}\nhost:storage.googleapis.com\n\nhost\nUNSIGNED-PAYLOAD");
    let to_sign = format!("GOOG4-RSA-SHA256\n{stamp}\n{scope}\n{}", hex(&Sha256::digest(canonical.as_bytes())));
    (format!("https://storage.googleapis.com{resource}?{query}"), to_sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_url_and_string_to_sign_follow_the_spec() {
        let now = DateTime::parse_from_rfc3339("2026-11-19T10:00:00Z").unwrap().with_timezone(&Utc);
        let (url, to_sign) = v4_request("GET", "payroll-exports", "org/payroll-input/p 1.json", "jobs@x.iam.gserviceaccount.com", now, 900);
        assert!(url.starts_with("https://storage.googleapis.com/payroll-exports/org/payroll-input/p%201.json?X-Goog-Algorithm=GOOG4-RSA-SHA256&"), "{url}");
        assert!(url.contains("X-Goog-Credential=jobs%40x.iam.gserviceaccount.com%2F20261119%2Fauto%2Fstorage%2Fgoog4_request"), "{url}");
        assert!(url.ends_with("X-Goog-Date=20261119T100000Z&X-Goog-Expires=900&X-Goog-SignedHeaders=host"), "{url}");
        let lines: Vec<&str> = to_sign.lines().collect();
        assert_eq!(&lines[..3], ["GOOG4-RSA-SHA256", "20261119T100000Z", "20261119/auto/storage/goog4_request"]);
        assert_eq!(lines[3].len(), 64, "hex sha256 of the canonical request");
    }
}
