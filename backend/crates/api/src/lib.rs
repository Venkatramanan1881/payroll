mod error;
mod extract;
mod idempotency;
mod routes;

pub use error::ApiError;
pub use extract::{authenticate, unix_now, Auth};

use axum::{routing::{get, post}, Json, Router};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Vec<u8>,
    pub phone_verifier: Arc<dyn domain_iam::firebase::PhoneTokenVerifier>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({ "status": "ok" })) }))
        .route("/v1/auth/firebase/sign-in", post(routes::auth::firebase_sign_in))
        .route("/v1/auth/organisation-choice", post(routes::auth::choose_organisation))
        .route("/v1/auth/refresh", post(routes::auth::refresh))
        .route("/v1/auth/sign-out", post(routes::auth::sign_out))
        .route("/v1/shifts", post(routes::sched::create_shift).get(routes::sched::list_shifts))
        .route("/v1/shifts/{id}", get(routes::sched::get_shift))
        .route("/v1/shifts/{id}/versions", post(routes::sched::create_shift_version))
        .route("/v1/shift-versions/{id}", get(routes::sched::get_shift_version).patch(routes::sched::update_shift_version))
        .route("/v1/shift-versions/{id}/publish", post(routes::sched::publish_shift_version))
        .route("/v1/weekly-off-patterns", post(routes::sched::create_pattern).get(routes::sched::list_patterns))
        .route("/v1/weekly-off-patterns/{id}/retire", post(routes::sched::retire_pattern))
        .route("/v1/holiday-calendars", post(routes::sched::create_calendar).get(routes::sched::list_calendars))
        .route("/v1/holiday-calendars/{id}", axum::routing::patch(routes::sched::update_calendar))
        .route("/v1/holiday-calendars/{id}/holidays", post(routes::sched::add_holiday).get(routes::sched::list_holidays))
        .route("/v1/holidays/{id}", axum::routing::patch(routes::sched::update_holiday))
        .route("/v1/holidays/{id}/remove", post(routes::sched::remove_holiday))
        .route("/v1/employee-schedules", post(routes::sched::assign_schedule).get(routes::sched::list_schedules))
        .route("/v1/roster-entries", get(routes::sched::list_roster))
        .route("/v1/roster-entries/set", post(routes::sched::set_roster))
        .route("/v1/me/roster", get(routes::sched::my_roster))
        .route("/v1/policy-sets", post(routes::policy::create_set).get(routes::policy::list_sets))
        .route("/v1/policy-sets/{id}", get(routes::policy::get_set))
        .route("/v1/policy-sets/{id}/versions", post(routes::policy::create_draft))
        .route("/v1/policy-versions/{id}", get(routes::policy::get_version).patch(routes::policy::update_version))
        .route("/v1/policy-versions/{id}/rule-groups/{rule_group}", get(routes::policy::get_rule_group).put(routes::policy::put_rule_group))
        .route("/v1/policy-versions/{id}/rule-groups/{rule_group}/switch", post(routes::policy::switch_rule_group))
        .route("/v1/policy-versions/{id}/publish", post(routes::policy::publish))
        .route("/v1/employees", post(routes::employees::create))
        .route("/v1/employees/{employee_id}", get(routes::employees::get).patch(routes::employees::patch))
        .route("/v1/employees/{employee_id}/assignments", get(routes::employees::list_assignments))
        .route("/v1/employees/{employee_id}/change-assignment", post(routes::employees::change_assignment))
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
}
