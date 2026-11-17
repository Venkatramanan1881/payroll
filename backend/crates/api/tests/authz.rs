mod common;

use api::{authenticate, ApiError, AppState};
use domain_iam::authz::{employee_in_scope, require, Scope};
use sqlx::PgPool;
use uuid::Uuid;

fn state(pool: PgPool) -> AppState {
    AppState { pool, jwt_secret: common::JWT_SECRET.to_vec() }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn no_token_401(pool: PgPool) {
    let r = authenticate(&state(pool), &axum::http::HeaderMap::new(), false).await;
    assert!(matches!(r, Err(ApiError::Unauthenticated)));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn valid_token_sets_principal(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let (p, _tx) = authenticate(&state(pool), &common::bearer(&s.hr_token), false).await.unwrap();
    assert_eq!((p.user_id, p.org_id), (s.hr_user_id, s.org_id));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn inactive_user_401(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    sqlx::query("UPDATE iam.user_account SET active = false WHERE id = $1").bind(s.hr_user_id).execute(&pool).await.unwrap();
    let r = authenticate(&state(pool), &common::bearer(&s.hr_token), false).await;
    assert!(matches!(r, Err(ApiError::Unauthenticated)));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn token_for_user_of_other_org_401(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    // A's user presented with B's organisation: RLS hides the user, so it must not authenticate.
    let forged = common::token(a.hr_user_id, b.org_id);
    let r = authenticate(&state(pool), &common::bearer(&forged), false).await;
    assert!(matches!(r, Err(ApiError::Unauthenticated)));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn suspended_org_refuses_writes_allows_reads(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    sqlx::query("UPDATE platform.organisation SET state = 'suspended' WHERE id = $1").bind(s.org_id).execute(&pool).await.unwrap();
    let st = state(pool);
    assert!(matches!(authenticate(&st, &common::bearer(&s.hr_token), true).await, Err(ApiError::OrgSuspended)));
    assert!(authenticate(&st, &common::bearer(&s.hr_token), false).await.is_ok());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn require_grant_and_scope(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let (p, mut tx) = authenticate(&state(pool), &common::bearer(&s.hr_token), false).await.unwrap();
    let g = require(&mut tx, &p, "WRK-01").await.unwrap();
    assert_eq!(g.scope, Scope::All);
    assert!(require(&mut tx, &p, "WRK-99").await.is_err());
}

#[test]
fn scope_branch_limits() {
    let (x, y, d) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    assert!(employee_in_scope(&Scope::All, y, None));
    assert!(employee_in_scope(&Scope::Branches(vec![x]), x, None));
    assert!(!employee_in_scope(&Scope::Branches(vec![x]), y, None));
    assert!(employee_in_scope(&Scope::Departments(vec![d]), y, Some(d)));
    assert!(!employee_in_scope(&Scope::Departments(vec![d]), y, None));
    assert!(!employee_in_scope(&Scope::Nothing, x, None));
}
