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
    /// Bearer secret for /internal/jobs; None disables the job endpoints.
    pub job_secret: Option<Vec<u8>>,
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
        .route("/v1/policy-assignments", post(routes::policy::create_assignment).get(routes::policy::list_assignments))
        .route("/v1/policy-assignments/{id}/end", post(routes::policy::end_assignment))
        .route("/v1/punches", post(routes::punches::record).get(routes::punches::list))
        .route("/v1/punches/sync", post(routes::punches::sync))
        .route("/v1/punches/{id}", get(routes::punches::get))
        .route("/v1/punch-attempts", get(routes::punches::attempts))
        .route("/v1/punch-flags", get(routes::punches::list_flags))
        .route("/v1/punch-flags/{id}/accept", post(routes::punches::accept_flag))
        .route("/v1/punch-flags/{id}/reject", post(routes::punches::reject_flag))
        .route("/v1/manual-attendance", post(routes::punches::manual))
        .route("/v1/attendance/employees/{employee_id}/days/{date}", get(routes::attendance::get_day))
        .route("/v1/attendance/daily", get(routes::attendance::daily))
        .route("/v1/attendance/recalculations", post(routes::attendance::recalculate))
        .route("/v1/attendance/regularisation-requests", post(routes::regularisation::submit).get(routes::regularisation::list))
        .route("/v1/attendance/regularisation-requests/pending-approval", get(routes::regularisation::pending))
        .route("/v1/attendance/regularisation-requests/{id}", get(routes::regularisation::get))
        .route("/v1/attendance/regularisation-requests/{id}/approve", post(routes::regularisation::approve))
        .route("/v1/attendance/regularisation-requests/{id}/reject", post(routes::regularisation::reject))
        .route("/v1/attendance/regularisation-requests/{id}/cancel", post(routes::regularisation::cancel))
        .route("/v1/attendance/hr-corrections", post(routes::regularisation::hr_correction))
        .route("/v1/attendance/bulk-regularisations", post(routes::regularisation::bulk))
        .route("/internal/jobs/{name}", post(routes::jobs::run))
        .route("/v1/organisation", get(routes::org::organisation))
        .route("/v1/branches", get(routes::org::list_branches).post(routes::org::create_branch))
        .route("/v1/branches/{id}", get(routes::org::get_branch).patch(routes::org::update_branch))
        .route("/v1/branches/{id}/geo-fences", get(routes::org::list_fences).post(routes::org::create_fence))
        .route("/v1/branches/{id}/ip-allow-list", get(routes::org::ip_list))
        .route("/v1/branches/{id}/ip-allow-list/change", post(routes::org::change_ip_list))
        .route("/v1/geo-fences/{id}", axum::routing::patch(routes::org::update_fence))
        .route("/v1/geo-fences/{id}/test", post(routes::org::test_fence))
        .route("/v1/geo-fences/{id}/switch-on", post(routes::org::switch_on))
        .route("/v1/geo-fences/{id}/switch-off", post(routes::org::switch_off))
        .route("/v1/lists/{kind}", get(routes::org::list_entries).post(routes::org::create_entry))
        .route("/v1/lists/{kind}/{id}", axum::routing::patch(routes::org::update_entry))
        .route("/v1/lists/{kind}/{id}/retire", post(routes::org::retire_entry))
        .route("/v1/contractors", get(routes::org::list_contractors).post(routes::org::create_contractor))
        .route("/v1/contractors/{id}", get(routes::org::get_contractor).patch(routes::org::update_contractor))
        .route("/v1/contractors/{id}/retire", post(routes::org::retire_contractor))
        .route("/v1/organisation-settings", get(routes::org::settings))
        .route("/v1/organisation-settings/versions", get(routes::org::settings_versions).post(routes::org::create_settings_version))
        .route("/v1/leave/types", get(routes::leave::list_types).post(routes::leave::create_type))
        .route("/v1/leave/types/{id}", get(routes::leave::get_type))
        .route("/v1/leave/types/{id}/versions", post(routes::leave::publish_version))
        .route("/v1/leave/employees/{employee_id}/balances", get(routes::leave::balances))
        .route("/v1/leave/employees/{employee_id}/ledger", get(routes::leave::ledger))
        .route("/v1/leave/employees/{employee_id}/ledger-adjustments", post(routes::leave::adjust))
        .route("/v1/leave/requests", post(routes::leave::submit).get(routes::leave::list))
        .route("/v1/leave/requests/on-behalf", post(routes::leave::submit_on_behalf))
        .route("/v1/leave/requests/bulk-decisions", post(routes::leave::bulk))
        .route("/v1/leave/requests/{id}", get(routes::leave::get))
        .route("/v1/leave/requests/{id}/approve", post(routes::leave::approve))
        .route("/v1/leave/requests/{id}/reject", post(routes::leave::reject))
        .route("/v1/leave/requests/{id}/cancel", post(routes::leave::cancel))
        .route("/v1/employees", post(routes::employees::create))
        .route("/v1/employees/{employee_id}", get(routes::employees::get).patch(routes::employees::patch))
        .route("/v1/employees/{employee_id}/assignments", get(routes::employees::list_assignments))
        .route("/v1/employees/{employee_id}/change-assignment", post(routes::employees::change_assignment))
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
}
