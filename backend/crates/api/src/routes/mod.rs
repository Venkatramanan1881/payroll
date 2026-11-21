pub mod attendance;
pub mod auth;
pub mod employees;
pub mod policy;
pub mod sched;
pub mod punches;
pub mod regularisation;
pub mod leave;
pub mod org;
pub mod jobs;
pub mod period;
pub mod iam;
pub mod vendor;
pub mod openbal;
pub mod support;
pub mod permissions;
pub mod onduty;
pub mod comp_off;
pub mod notifications;
pub mod reports;
pub mod dashboards;

/// Adds `employee_code` and `employee_name` to every item that carries an `employee_id`, for display. The items were
/// already filtered to the caller's scope; this only names them (one query, under the organisation's RLS).
pub(crate) async fn with_names(tx: &mut db::OrgTx<'_>, items: &mut [serde_json::Value]) -> Result<(), crate::ApiError> {
    let ids: Vec<uuid::Uuid> = items.iter().filter_map(|i| i["employee_id"].as_str().and_then(|s| s.parse().ok())).collect();
    if ids.is_empty() {
        return Ok(());
    }
    let rows: Vec<(uuid::Uuid, String, String)> =
        sqlx::query_as("SELECT id, employee_code, full_name FROM workforce.employee WHERE id = ANY($1)").bind(&ids).fetch_all(&mut ***tx).await?;
    let names: std::collections::HashMap<String, (String, String)> = rows.into_iter().map(|(id, c, n)| (id.to_string(), (c, n))).collect();
    for item in items.iter_mut() {
        if let Some((code, name)) = item["employee_id"].as_str().and_then(|id| names.get(id)) {
            item["employee_code"] = serde_json::json!(code);
            item["employee_name"] = serde_json::json!(name);
        }
    }
    Ok(())
}
