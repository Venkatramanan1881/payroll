//! Admin password sign-in (UM-R3) and the AC-08 second step (D-23): candidates by email across organisations,
//! Argon2id verification, and the per-organisation second-step state.

use crate::signin::Candidate;
use argon2::password_hash::{PasswordHash, PasswordVerifier};
use sqlx::PgPool;
use uuid::Uuid;

fn dummy_hash() -> &'static str {
    static H: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    H.get_or_init(|| {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        argon2::Argon2::default().hash_password(b"not a password", &SaltString::generate(&mut OsRng)).expect("hash").to_string()
    })
}

/// Failed second-step codes for a user in the last 15 minutes (kept in the audit log, so every instance sees them).
pub const MAX_FAILED_CODES: i64 = 5;

pub async fn failed_codes(pool: &PgPool, org: Uuid, user: Uuid) -> Result<i64, db::DomainError> {
    let mut tx = db::begin_org(pool, org).await?;
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM audit.audit_entry WHERE action = 'iam.second_factor_failed' AND target_id = $1 AND occurred_at > now() - interval '15 minutes'",
    )
    .bind(user)
    .fetch_one(&mut **tx)
    .await?)
}

/// Records a failed code in its own transaction (the request itself fails).
pub async fn record_failed_code(pool: &PgPool, org: Uuid, user: Uuid) -> Result<(), db::DomainError> {
    let mut tx = db::begin_org(pool, org).await?;
    db::audit(&mut tx, user, "iam.second_factor_failed", "user_account", Some(user), None).await?;
    tx.commit().await?;
    Ok(())
}

/// Marks a code's step as used; false when that step (or a later one) was already used: the code is not accepted again.
pub async fn claim_step(pool: &PgPool, org: Uuid, user: Uuid, step: i64) -> Result<bool, db::DomainError> {
    let mut tx = db::begin_org(pool, org).await?;
    let claimed = sqlx::query("UPDATE iam.user_account SET mfa_last_step = $2 WHERE id = $1 AND (mfa_last_step IS NULL OR mfa_last_step < $2)")
        .bind(user)
        .bind(step)
        .execute(&mut **tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(claimed == 1)
}

/// Associated data binding an encrypted secret to its organisation and user (a copied ciphertext does not decrypt).
pub fn secret_aad(org: Uuid, user: Uuid) -> Vec<u8> {
    format!("{org}:{user}").into_bytes()
}

fn matches(hash: &str, password: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| argon2::Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

/// Active password users with this email whose password matches, across organisations (iam.resolve_login runs
/// before any organisation is known). A wrong password and an unknown email look the same to the caller.
// shortcut: no lock-out after repeated failures yet (OQ-C03-06); add a counter before going live.
pub async fn candidates(pool: &PgPool, email: &str, password: &str) -> sqlx::Result<Vec<Candidate>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut *tx).await?;
    let pairs: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT org_id, user_id FROM iam.resolve_login(NULL, $1) ORDER BY org_id").bind(email).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let mut out = vec![];
    let mut checked = false;
    for (org_id, user_id) in pairs {
        let mut tx = db::begin_org(pool, org_id).await?;
        let row: Option<(Option<String>, String)> = sqlx::query_as(
            "SELECT u.password_hash, o.display_name FROM iam.user_account u JOIN platform.organisation o ON o.id = u.org_id
             WHERE u.id = $1 AND u.active AND u.login_kind = 'password'",
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some((Some(hash), display_name)) = row {
            checked = true;
            if matches(&hash, password) {
                out.push(Candidate { org_id, user_id, display_name });
            }
        }
    }
    if !checked {
        // An unknown email costs the same Argon2 work as a known one, so timing does not reveal accounts.
        matches(dummy_hash(), password);
    }
    Ok(out)
}

/// (AC-08 on for the organisation, the user's encrypted authenticator secret).
pub async fn second_step(pool: &PgPool, org: Uuid, user: Uuid) -> Result<(bool, Option<Vec<u8>>), db::DomainError> {
    let mut tx = db::begin_org(pool, org).await?;
    let on = crate::admin::access_settings(&mut tx).await?["admin_second_step"].as_bool().unwrap_or(true);
    let secret: Option<Vec<u8>> = sqlx::query_scalar("SELECT mfa_secret_enc FROM iam.user_account WHERE id = $1 AND active").bind(user).fetch_optional(&mut **tx).await?.flatten();
    Ok((on, secret))
}

/// Stores the first secret once its first code was right; a secret already stored is never replaced here.
pub async fn store_secret(pool: &PgPool, org: Uuid, user: Uuid, sealed: &[u8]) -> Result<bool, db::DomainError> {
    let mut tx = db::begin_org(pool, org).await?;
    let stored = sqlx::query("UPDATE iam.user_account SET mfa_secret_enc = $2 WHERE id = $1 AND active AND mfa_secret_enc IS NULL")
        .bind(user)
        .bind(sealed)
        .execute(&mut **tx)
        .await?
        .rows_affected();
    if stored == 1 {
        db::audit(&mut tx, user, "iam.second_factor_enrolled", "user_account", Some(user), None).await?;
    }
    tx.commit().await?;
    Ok(stored == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};

    #[test]
    fn argon2_round_trip() {
        let h = argon2::Argon2::default().hash_password(b"correct horse", &SaltString::generate(&mut OsRng)).unwrap().to_string();
        assert!(matches(&h, "correct horse") && !matches(&h, "wrong") && !matches("not-a-hash", "x"));
    }
}
