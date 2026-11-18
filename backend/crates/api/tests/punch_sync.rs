mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const LAT: f64 = 12.9716;
const LON: f64 = 77.5946;

async fn setup(pool: &PgPool) -> (common::SeedOrg, axum::Router, common::Worker) {
    let s = common::seed_org(pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    common::fence(pool, s.org_id, s.branch_id, LAT, LON, 200).await;
    (s, app, w)
}

/// An offline item with its phone clock time and uptime reading at capture.
fn item(at: DateTime<Utc>, uptime_ms: i64) -> Value {
    json!({
        "idempotency_key": Uuid::new_v4(),
        "client_time": at,
        "captured_tz": "Asia/Kolkata",
        "device_uptime_ms": uptime_ms,
        "location": { "latitude": LAT, "longitude": LON, "accuracy_m": 10 },
    })
}

async fn sync(app: &axum::Router, w: &common::Worker, sync_uptime_ms: i64, items: Vec<Value>) -> (StatusCode, Value) {
    let body = json!({ "sync_client_time": Utc::now(), "sync_uptime_ms": sync_uptime_ms, "punches": items });
    common::as_user(app, &w.token, "POST", "/v1/punches/sync", Some(body), &[]).await
}

fn outcomes(r: &Value) -> Vec<String> {
    r["results"].as_array().unwrap().iter().map(|x| x["outcome"].as_str().unwrap().to_string()).collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retried_batch_reports_duplicates_and_stores_nothing_twice(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let now = Utc::now();
    let up = 10_000_000;
    let items = vec![item(now - Duration::hours(2), up - 7_200_000), item(now - Duration::hours(1), up - 3_600_000)];
    let (st, first) = sync(&app, &w, up, items.clone()).await;
    assert_eq!(st, StatusCode::OK, "{first}");
    assert_eq!(outcomes(&first), ["accepted", "accepted"], "{first}");
    let (_, again) = sync(&app, &w, up + 1000, items).await;
    assert_eq!(outcomes(&again), ["duplicate", "duplicate"]);
    assert_eq!(again["results"][0]["code"], "DUPLICATE_PUNCH");
    assert_eq!(again["results"][0]["punch_id"], first["results"][0]["punch_id"]);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 2);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn late_sync_after_pc07_days(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let now = Utc::now();
    let up = 900_000_000;
    let (_, r) = sync(&app, &w, up, vec![item(now - Duration::days(4), up - 4 * 86_400_000)]).await;
    assert_eq!(outcomes(&r), ["flagged"], "{r}");
    assert_eq!(r["results"][0]["flags"], json!(["late_sync"]));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn changed_phone_clock_and_unmeasurable_restart_are_flagged(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let now = Utc::now();
    let up = 10_000_000;
    // Captured 1 hour ago by the uptime clock, but the phone clock said 20 minutes later than that.
    let moved = item(now - Duration::hours(1) + Duration::minutes(20), up - 3_600_000);
    // Uptime at capture larger than at sync: the phone restarted, so the clock cannot be checked.
    let rebooted = item(now - Duration::minutes(30), up + 5_000_000);
    let (_, r) = sync(&app, &w, up, vec![moved, rebooted]).await;
    assert_eq!(r["results"][0]["flags"], json!(["clock_mismatch"]), "{r}");
    assert_eq!(r["results"][1]["flags"], json!(["clock_mismatch"]), "a restart cannot hide a changed clock: {r}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn one_bad_item_does_not_fail_the_batch_and_order_is_kept(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let now = Utc::now();
    let up = 10_000_000;
    let mut bad_tz = item(now - Duration::minutes(10), up - 600_000);
    bad_tz["captured_tz"] = json!("Mars/Olympus");
    let unknown_field = json!({ "idempotency_key": Uuid::new_v4(), "inside_fence": true });
    let good = item(now - Duration::minutes(5), up - 300_000);
    let (st, r) = sync(&app, &w, up, vec![bad_tz, unknown_field, good]).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!(outcomes(&r), ["refused", "refused", "accepted"]);
    assert_eq!(r["results"][0]["refusal"]["code"], "VALIDATION_FAILED");
    assert_eq!(r["results"][1]["refusal"]["code"], "VALIDATION_FAILED");
    let (st, _) = sync(&app, &w, up, vec![]).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "an empty batch is a malformed envelope");
}

/// Review focus 4: the off-day check uses the local date in captured_tz, not the UTC date.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn near_midnight_uses_local_date(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let cal = Uuid::new_v4();
    sqlx::query("INSERT INTO sched.holiday_calendar (org_id, id, name) VALUES ($1, $2, 'HQ')").bind(s.org_id).bind(cal).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO sched.holiday (org_id, calendar_id, holiday_date, name, kind) VALUES ($1, $2, '2026-03-10', 'Festival', 'festival')")
        .bind(s.org_id).bind(cal).execute(&pool).await.unwrap();
    sqlx::query("UPDATE org.branch SET holiday_calendar_id = $2 WHERE id = $1").bind(s.branch_id).bind(cal).execute(&pool).await.unwrap();
    // 19:00 UTC on 9 March = 00:30 on 10 March in India: a holiday there, though not on the UTC date.
    let local_holiday: DateTime<Utc> = "2026-03-09T19:00:00Z".parse().unwrap();
    // 18:00 UTC on 9 March = 23:30 on 9 March in India: a working day.
    let local_workday: DateTime<Utc> = "2026-03-09T18:00:00Z".parse().unwrap();
    let up = 100_000_000_000; // ~3 years of uptime, longer than the punch age
    let age = |t: DateTime<Utc>| (Utc::now() - t).num_milliseconds();
    let (_, r) = sync(&app, &w, up, vec![item(local_holiday, up - age(local_holiday)), item(local_workday, up - age(local_workday))]).await;
    let flags_of = |r: &Value, i: usize| r["results"][i]["flags"].as_array().unwrap().iter().map(|f| f.as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert!(flags_of(&r, 0).contains(&"no_shift_or_off_day".to_string()), "{r}");
    assert!(!flags_of(&r, 1).contains(&"no_shift_or_off_day".to_string()), "{r}");
    // A client zone far from the branch cannot move the punch off the holiday: the branch zone decides the date.
    let mut far = item(local_holiday, up - age(local_holiday));
    far["captured_tz"] = json!("Pacific/Pago_Pago");
    let (_, r) = sync(&app, &w, up, vec![far]).await;
    assert!(flags_of(&r, 0).contains(&"no_shift_or_off_day".to_string()), "{r}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn future_items_and_missing_location_are_refused_once(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let now = Utc::now();
    let up = 10_000_000;
    let future = item(now + Duration::days(1), up);
    let mut no_location = item(now - Duration::hours(1), up - 3_600_000);
    no_location["location"] = Value::Null;
    let (_, r) = sync(&app, &w, up, vec![future, no_location.clone()]).await;
    assert_eq!(r["results"][0]["refusal"]["code"], "VALIDATION_FAILED", "{r}");
    assert_eq!((r["results"][1]["refusal"]["code"].as_str(), r["results"][1]["refusal"]["rule_ref"].as_str()), (Some("PUNCH_REFUSED"), Some("PC-02")), "{r}");
    sync(&app, &w, up + 1000, vec![no_location.clone()]).await;
    let (n, at): (i64, Option<DateTime<Utc>>) = sqlx::query_as("SELECT count(*), max(attempted_at) FROM timecap.punch_attempt_refused").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1, "a retried batch does not record the refusal twice");
    let captured: DateTime<Utc> = serde_json::from_value(no_location["client_time"].clone()).unwrap();
    assert_eq!(at.map(|t| t.timestamp_millis()), Some(captured.timestamp_millis()), "attempted_at is when the employee tried");
}
