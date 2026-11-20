//! GET /v1/employees (WRK-08): scope, filters, sort and cursor paging (web slice 1).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn add(app: &axum::Router, s: &common::SeedOrg, branch: Uuid, name: &str) {
    let body = json!({ "full_name": name, "joining_date": "2026-11-01", "employment_status": "active", "master_branch_id": branch });
    let (st, b) = common::as_user(app, &s.hr_token, "POST", "/v1/employees", Some(body), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
}

async fn get(app: &axum::Router, token: &str, uri: &str) -> (StatusCode, Value) {
    common::as_user(app, token, "GET", uri, None, &[]).await
}

fn names(v: &Value) -> Vec<String> {
    v["items"].as_array().unwrap().iter().map(|i| i["full_name"].as_str().unwrap().to_string()).collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_scoped_filtered_paged(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let br2 = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'BR2', 'Branch 2')")
        .bind(s.org_id).bind(br2).execute(&pool).await.unwrap();
    let app = common::app(pool.clone());
    add(&app, &s, s.branch_id, "Charu").await;
    add(&app, &s, s.branch_id, "Asha").await;
    add(&app, &s, br2, "Bala").await;
    let hr = s.hr_token.clone();

    let (st, all) = get(&app, &hr, "/v1/employees?sort=full_name").await;
    assert_eq!((st, names(&all), all["next_cursor"].clone()), (StatusCode::OK, vec!["Asha".into(), "Bala".into(), "Charu".into()], Value::Null), "{all}");
    assert_eq!(all["items"][0]["master_branch_id"], json!(s.branch_id));
    assert_eq!(names(&get(&app, &hr, "/v1/employees?q=al").await.1), ["Bala"]);
    assert!(names(&get(&app, &hr, "/v1/employees?q=%25").await.1).is_empty(), "wildcards are literal");
    assert_eq!(names(&get(&app, &hr, &format!("/v1/employees?branch_id={br2}")).await.1), ["Bala"]);
    assert!(names(&get(&app, &hr, "/v1/employees?employment_status=exited").await.1).is_empty());

    let (_, p1) = get(&app, &hr, "/v1/employees?sort=-full_name&limit=2").await;
    assert_eq!(names(&p1), ["Charu", "Bala"]);
    let (_, p2) = get(&app, &hr, &format!("/v1/employees?sort=-full_name&limit=2&cursor={}", p1["next_cursor"].as_str().unwrap())).await;
    assert_eq!((names(&p2), p2["next_cursor"].clone()), (vec!["Asha".to_string()], Value::Null));
    assert_eq!(get(&app, &hr, "/v1/employees?cursor=nope").await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(get(&app, &hr, "/v1/employees?sort=salary").await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(get(&app, &hr, "/v1/employees?limit=500").await.0, StatusCode::UNPROCESSABLE_ENTITY);

    // A branch-scoped HR user sees only that branch (AC-C02-01.6).
    sqlx::query("DELETE FROM iam.user_scope WHERE user_id = $1").bind(s.hr_user_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)")
        .bind(s.org_id).bind(s.hr_user_id).bind(br2).execute(&pool).await.unwrap();
    assert_eq!(names(&get(&app, &hr, "/v1/employees").await.1), ["Bala"]);

    // An employee with WRK-08 Self sees their own record only; without WRK-08, 403.
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(s.org_id).bind(s.hr_user_id).execute(&pool).await.unwrap();
    let w = common::seed_worker(&pool, &app, &s, "Devi").await;
    assert_eq!(get(&app, &w.token, "/v1/employees").await.0, StatusCode::FORBIDDEN);
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) SELECT org_id, role_id, 'WRK-08', 'self' FROM iam.user_role WHERE user_id = $1")
        .bind(w.user_id).execute(&pool).await.unwrap();
    assert_eq!(names(&get(&app, &w.token, "/v1/employees").await.1), ["Devi"]);
}

/// Another organisation's employees never appear; `_` is literal; bad inputs are 422; approve-only grants list nobody.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_isolated_and_validated(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    add(&app, &a, a.branch_id, "Asha_A").await;
    add(&app, &b, b.branch_id, "Bala B").await;
    assert_eq!(names(&get(&app, &a.hr_token, "/v1/employees").await.1), ["Asha_A"]);
    assert_eq!(names(&get(&app, &b.hr_token, "/v1/employees").await.1), ["Bala B"]);
    assert_eq!(names(&get(&app, &a.hr_token, "/v1/employees?q=a_a").await.1), ["Asha_A"]);
    assert!(names(&get(&app, &a.hr_token, "/v1/employees?q=sh_").await.1).is_empty(), "_ is not a wildcard");
    for bad in ["employment_status=gone", "branch_id=not-a-uuid", "limit=0", "cursor=-1"] {
        assert_eq!(get(&app, &a.hr_token, &format!("/v1/employees?{bad}")).await.0, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    sqlx::query("UPDATE iam.role_grant SET grant_kind = 'approve' WHERE operation_code = 'WRK-08' AND org_id = $1").bind(a.org_id).execute(&pool).await.unwrap();
    let (st, none) = get(&app, &a.hr_token, "/v1/employees").await;
    assert_eq!((st, names(&none)), (StatusCode::OK, Vec::<String>::new()));
    // A self grant with no employee record lists nobody.
    sqlx::query("UPDATE iam.role_grant SET grant_kind = 'self' WHERE operation_code = 'WRK-08' AND org_id = $1").bind(a.org_id).execute(&pool).await.unwrap();
    assert!(names(&get(&app, &a.hr_token, "/v1/employees").await.1).is_empty());
}
