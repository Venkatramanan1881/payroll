//! Authenticator-app codes for the AC-08 second step (D-23): RFC 6238 TOTP (HMAC-SHA1, 30 s, 6 digits) and
//! AES-256-GCM encryption of the shared secret at rest (`iam.user_account.mfa_secret_enc`).

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;

pub const STEP_SECS: i64 = 30;

/// A new 160-bit shared secret.
pub fn new_secret() -> Vec<u8> {
    let mut s = vec![0u8; 20];
    rand::thread_rng().fill_bytes(&mut s);
    s
}

/// RFC 4648 base32 without padding (the form authenticator apps read).
pub fn base32(bytes: &[u8]) -> String {
    const A: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let (mut out, mut buf, mut bits) = (String::new(), 0u32, 0u32);
    for &b in bytes {
        buf = (buf << 8) | u32::from(b);
        bits += 8;
        while bits >= 5 {
            out.push(A[((buf >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
    }
    if bits > 0 {
        out.push(A[((buf << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// The code for a time step (RFC 4226 dynamic truncation).
pub fn code_at(secret: &[u8], step: i64) -> String {
    let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(secret).expect("HMAC takes any key length");
    mac.update(&(step as u64).to_be_bytes());
    let h = mac.finalize().into_bytes();
    let o = (h[19] & 0x0f) as usize;
    let n = (u32::from(h[o] & 0x7f) << 24) | (u32::from(h[o + 1]) << 16) | (u32::from(h[o + 2]) << 8) | u32::from(h[o + 3]);
    format!("{:06}", n % 1_000_000)
}

/// Accepts the current step and one either side (clock drift), comparing in constant time.
pub fn verify(secret: &[u8], code: &str, now_secs: i64) -> bool {
    let step = now_secs.div_euclid(STEP_SECS);
    code.len() == 6 && (-1..=1).fold(false, |ok, d| ok | same(code_at(secret, step + d).as_bytes(), code.as_bytes()))
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// `otpauth://` URI for the enrolment QR code.
pub fn otpauth_uri(secret: &[u8], account: &str) -> String {
    let enc = |s: &str| s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>();
    format!("otpauth://totp/Payroll:{}?secret={}&issuer=Payroll&algorithm=SHA1&digits=6&period=30", enc(account), base32(secret))
}

/// nonce (12 bytes) || ciphertext; `aad` binds it to its owner (organisation and user).
pub fn encrypt(key: &[u8; 32], plain: &[u8], aad: &[u8]) -> Vec<u8> {
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let mut out = nonce.to_vec();
    out.extend(Aes256Gcm::new(key.into()).encrypt(Nonce::from_slice(&nonce), Payload { msg: plain, aad }).expect("AES-GCM encryption cannot fail"));
    out
}

pub fn decrypt(key: &[u8; 32], sealed: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < 12 {
        return None;
    }
    let (nonce, ct) = sealed.split_at(12);
    Aes256Gcm::new(key.into()).decrypt(Nonce::from_slice(nonce), Payload { msg: ct, aad }).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6238_vectors() {
        // RFC 6238 appendix B, SHA-1 secret "12345678901234567890" (8-digit values; the 6-digit codes are their tails).
        let s = b"12345678901234567890";
        assert_eq!(code_at(s, 59 / 30), "287082");
        assert_eq!(code_at(s, 1111111109 / 30), "081804");
        assert_eq!(code_at(s, 1234567890 / 30), "005924");
        assert_eq!(code_at(s, 20000000000 / 30), "353130");
    }

    #[test]
    fn verify_allows_one_step_of_drift_only() {
        let s = new_secret();
        let now = 1_700_000_000;
        assert!(verify(&s, &code_at(&s, now / 30 - 1), now));
        assert!(verify(&s, &code_at(&s, now / 30 + 1), now));
        assert!(!verify(&s, &code_at(&s, now / 30 - 3), now));
        assert!(!verify(&s, "12345", now));
    }

    #[test]
    fn base32_and_encryption_round_trip() {
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI"); // RFC 4648 test vector
        let key = [7u8; 32];
        let sealed = encrypt(&key, b"secret", b"org:user");
        assert_eq!(decrypt(&key, &sealed, b"org:user").as_deref(), Some(&b"secret"[..]));
        assert!(decrypt(&[8u8; 32], &sealed, b"org:user").is_none());
        assert!(decrypt(&key, &sealed, b"org:other").is_none(), "bound to its owner");
        assert!(otpauth_uri(b"x", "hr@example.in").starts_with("otpauth://totp/Payroll:hr%40example.in?secret="));
    }
}
