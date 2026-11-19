//! Admin password sign-in and the AC-08 authenticator step (slice 11, D-23).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// A password admin of the seeded organisation (the hash is made by the API, as for any user).
async fn admin(pool: &PgPool, org: Uuid, email: &str) -> Uuid {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let hash = argon2::Argon2::default().hash_password(b"correct horse battery", &SaltString::generate(&mut OsRng)).unwrap().to_string();
    sqlx::query_scalar("INSERT INTO iam.user_account (org_id, login_kind, email, password_hash) VALUES ($1, 'password', $2, $3) RETURNING id")
        .bind(org).bind(email).bind(hash).fetch_one(pool).await.unwrap()
}

async fn post(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let (res, b) = common::send(app, common::post_json(uri, body)).await;
    (res.status(), b)
}

fn base32_decode(s: &str) -> Vec<u8> {
    let (mut out, mut buf, mut bits) = (vec![], 0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'2'..=b'7' => c - b'2' + 26,
            _ => continue,
        };
        buf = (buf << 5) | u32::from(v);
        bits += 5;
        if bits >= 8 {
            out.push((buf >> (bits - 8)) as u8);
            bits -= 8;
        }
    }
    out
}

fn now() -> i64 {
    common::now()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn password_then_authenticator(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let user = admin(&pool, s.org_id, "admin@sribakery.in").await;
    let creds = json!({ "email": "Admin@SriBakery.in", "password": "correct horse battery" });

    assert_eq!(post(&app, "/v1/auth/password/sign-in", json!({ "email": "admin@sribakery.in", "password": "wrong" })).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(post(&app, "/v1/auth/password/sign-in", json!({ "email": "nobody@sribakery.in", "password": "x" })).await.0, StatusCode::UNAUTHORIZED);

    // First sign-in (AC-08 on by default): a challenge and the secret to enrol; nothing stored yet.
    let (st, first) = post(&app, "/v1/auth/password/sign-in", creds.clone()).await;
    assert_eq!((st, first["code"].as_str()), (StatusCode::UNAUTHORIZED, Some("SECOND_FACTOR_REQUIRED")), "{first}");
    let uri = first["otpauth_uri"].as_str().unwrap();
    let secret = base32_decode(uri.split("secret=").nth(1).unwrap().split('&').next().unwrap());
    let challenge = first["challenge_token"].as_str().unwrap();
    assert_eq!(common::as_user(&app, challenge, "GET", "/v1/me", None, &[]).await.0, StatusCode::UNAUTHORIZED, "a challenge is not an access token");
    let stored: Option<Vec<u8>> = sqlx::query_scalar("SELECT mfa_secret_enc FROM iam.user_account WHERE id = $1").bind(user).fetch_one(&pool).await.unwrap();
    assert!(stored.is_none(), "stored only after the first right code");

    let wrong = format!("{:06}", (domain_iam::totp::code_at(&secret, now() / 30).parse::<u32>().unwrap() + 1) % 1_000_000);
    let (st, b) = post(&app, "/v1/auth/second-factor/verify", json!({ "challenge_token": challenge, "code": wrong })).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::BAD_REQUEST, Some("OTP_INVALID")));
    let (st, tokens) = post(&app, "/v1/auth/second-factor/verify", json!({ "challenge_token": challenge, "code": domain_iam::totp::code_at(&secret, now() / 30) })).await;
    assert_eq!(st, StatusCode::OK, "{tokens}");
    assert_eq!(common::as_user(&app, tokens["access_token"].as_str().unwrap(), "GET", "/v1/me", None, &[]).await.0, StatusCode::OK);
    let stored: Option<Vec<u8>> = sqlx::query_scalar("SELECT mfa_secret_enc FROM iam.user_account WHERE id = $1").bind(user).fetch_one(&pool).await.unwrap();
    let stored = stored.expect("enrolled");
    assert_ne!(stored, secret, "encrypted at rest");
    let aad = domain_iam::password::secret_aad(s.org_id, user);
    assert_eq!(domain_iam::totp::decrypt(&common::MFA_KEY, &stored, &aad).unwrap(), secret);

    // Later sign-ins: a challenge only, checked against the stored secret; a fresh enrolment cannot replace it.
    let (_, second) = post(&app, "/v1/auth/password/sign-in", creds.clone()).await;
    assert!(second.get("otpauth_uri").is_none(), "{second}");
    let (st, _) = post(&app, "/v1/auth/second-factor/verify", json!({ "challenge_token": second["challenge_token"], "code": domain_iam::totp::code_at(&secret, now() / 30) })).await;
    assert_eq!(st, StatusCode::OK);
    let other = domain_iam::totp::new_secret();
    use base64::Engine;
    let forged_pending = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(domain_iam::totp::encrypt(&common::MFA_KEY, &other, &aad));
    let forged = domain_iam::tokens::issue_challenge(common::JWT_SECRET, user, s.org_id, Some(forged_pending), now());
    let (st, _) = post(&app, "/v1/auth/second-factor/verify", json!({ "challenge_token": forged, "code": domain_iam::totp::code_at(&other, now() / 30) })).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "the stored secret always wins over a pending one");

    // AC-08 off: tokens straight away.
    let off = json!({ "admin_second_step": false, "languages": ["en"] });
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2026-01-01', $2, 1)")
        .bind(s.org_id).bind(off).execute(&pool).await.unwrap();
    let (st, direct) = post(&app, "/v1/auth/password/sign-in", creds).await;
    assert!(st == StatusCode::OK && direct["access_token"].is_string(), "{direct}");
}

/// Review fixes: guesses are capped; phone sign-in never opens a password admin; the organisation choice keeps AC-08.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn second_step_cannot_be_skipped_or_guessed(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let user = admin(&pool, s.org_id, "admin@sribakery.in").await;
    sqlx::query("UPDATE iam.user_account SET mobile = '+919811100009' WHERE id = $1").bind(user).execute(&pool).await.unwrap();
    let (res, _) = common::send(&app, common::post_json("/v1/auth/firebase/sign-in", json!({ "firebase_id_token": "phone:+919811100009" }))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "a password admin never signs in by phone code alone");

    let creds = json!({ "email": "admin@sribakery.in", "password": "correct horse battery" });
    let (_, first) = post(&app, "/v1/auth/password/sign-in", creds.clone()).await;
    let challenge = first["challenge_token"].as_str().unwrap().to_string();
    for _ in 0..5 {
        assert_eq!(post(&app, "/v1/auth/second-factor/verify", json!({ "challenge_token": challenge, "code": "000000" })).await.0, StatusCode::BAD_REQUEST);
    }
    let uri = first["otpauth_uri"].as_str().unwrap();
    let secret = base32_decode(uri.split("secret=").nth(1).unwrap().split('&').next().unwrap());
    let (st, b) = post(&app, "/v1/auth/second-factor/verify", json!({ "challenge_token": challenge, "code": domain_iam::totp::code_at(&secret, now() / 30) })).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::TOO_MANY_REQUESTS, Some("RATE_LIMITED")), "even the right code waits after five wrong ones");

    // The same email in a second organisation: the choice still leads to that organisation's second step.
    let other = common::seed_org(&pool).await;
    admin(&pool, other.org_id, "admin@sribakery.in").await;
    let (st, choice) = post(&app, "/v1/auth/password/sign-in", creds).await;
    assert_eq!((st, choice["code"].as_str()), (StatusCode::CONFLICT, Some("ORGANISATION_CHOICE_REQUIRED")), "{choice}");
    let (st, chosen) = post(&app, "/v1/auth/organisation-choice", json!({ "choice_token": choice["choice_token"], "candidate_ref": "1" })).await;
    assert_eq!((st, chosen["code"].as_str()), (StatusCode::UNAUTHORIZED, Some("SECOND_FACTOR_REQUIRED")), "{chosen}");
}
