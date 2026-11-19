//! Sessions for sign-in (D-12): candidate lookup by verified phone, session open / rotate / revoke.

use crate::tokens::{new_refresh, parse_refresh};
use sqlx::PgPool;
use uuid::Uuid;

pub const REFRESH_TTL_DAYS: i32 = 30;

pub struct Candidate {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Active users with this mobile number, across organisations (iam.resolve_login runs before any organisation is known).
pub async fn candidates(pool: &PgPool, mobile: &str) -> sqlx::Result<Vec<Candidate>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut *tx).await?;
    let pairs: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT org_id, user_id FROM iam.resolve_login($1, NULL) ORDER BY org_id")
        .bind(mobile)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    let mut out = Vec::with_capacity(pairs.len());
    for (org_id, user_id) in pairs {
        let mut tx = db::begin_org(pool, org_id).await?;
        // Only phone users sign in by phone: a password admin with a mobile must still pass the password and AC-08.
        let display_name: Option<String> = sqlx::query_scalar(
            "SELECT o.display_name FROM platform.organisation o JOIN iam.user_account u ON u.org_id = o.id WHERE o.id = $1 AND u.id = $2 AND u.login_kind = 'mobile_otp'",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(display_name) = display_name {
            out.push(Candidate { org_id, user_id, display_name });
        }
    }
    Ok(out)
}

/// Opens a session for an active user; returns (session id, refresh token). None if the user is not active.
pub async fn open_session(pool: &PgPool, org_id: Uuid, user_id: Uuid, device_label: Option<&str>) -> sqlx::Result<Option<(Uuid, String)>> {
    let mut tx = db::begin_org(pool, org_id).await?;
    let active: Option<bool> = sqlx::query_scalar("UPDATE iam.user_account SET last_sign_in_at = now() WHERE id = $1 AND active RETURNING active")
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
    if active.is_none() {
        return Ok(None);
    }
    let (refresh, hash) = new_refresh(org_id);
    let sid: Uuid = sqlx::query_scalar(
        "INSERT INTO iam.session (org_id, user_id, refresh_token_hash, device_label, expires_at)
         VALUES (app.current_org(), $1, $2, $3, now() + make_interval(days => $4)) RETURNING id",
    )
    .bind(user_id)
    .bind(hex(&hash))
    .bind(device_label)
    .bind(REFRESH_TTL_DAYS)
    .fetch_one(&mut **tx)
    .await?;
    db::audit(&mut tx, user_id, "auth.sign_in", "session", Some(sid), None).await?;
    tx.commit().await?;
    Ok(Some((sid, refresh)))
}

/// Rotates a refresh token: the presented one stops working. Returns (org, user, session, new refresh token).
pub async fn rotate_session(pool: &PgPool, refresh_token: &str) -> sqlx::Result<Option<(Uuid, Uuid, Uuid, String)>> {
    let Some((org_id, hash)) = parse_refresh(refresh_token) else { return Ok(None) };
    let mut tx = db::begin_org(pool, org_id).await?;
    let (new_token, new_hash) = new_refresh(org_id);
    let row: Option<(Uuid, Uuid)> = sqlx::query_as(
        "UPDATE iam.session s SET refresh_token_hash = $2, expires_at = now() + make_interval(days => $3)
         FROM iam.user_account u
         WHERE s.refresh_token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now()
           AND u.org_id = s.org_id AND u.id = s.user_id AND u.active
         RETURNING s.user_id, s.id",
    )
    .bind(hex(&hash))
    .bind(hex(&new_hash))
    .bind(REFRESH_TTL_DAYS)
    .fetch_optional(&mut **tx)
    .await?;
    tx.commit().await?;
    Ok(row.map(|(user_id, sid)| (org_id, user_id, sid, new_token)))
}

/// Revokes one session of the caller (sign-out).
pub async fn revoke_session(tx: &mut db::OrgTx<'_>, user_id: Uuid, sid: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE iam.session SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL")
        .bind(sid)
        .bind(user_id)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, user_id, "auth.sign_out", "session", Some(sid), None).await
}
