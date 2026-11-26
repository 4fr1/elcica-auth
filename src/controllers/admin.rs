use crate::models::{plans, roles, usages, user_profiles, users};
use axum::{body::Body as AxumBody, response::Response};
use hyper::{header::CONTENT_TYPE, StatusCode};
use loco_rs::prelude::*;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

fn json_response(status: StatusCode, value: Value) -> Result<Response> {
    let bytes = serde_json::to_vec(&value)?;
    Ok(Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(AxumBody::from(bytes))?)
}

#[derive(Debug, Serialize)]
struct CountRow {
    name: String,
    total: usize,
}

#[derive(Debug, Serialize)]
struct UserRow {
    pid: String,
    name: String,
    email: String,
    role: String,
    plan: String,
    sent: i64,
    quota: i64,
    joined: String,
}

#[derive(Debug, Serialize)]
struct Overview {
    total_users: usize,
    roles: Vec<CountRow>,
    plans: Vec<CountRow>,
    period: String,
    sent_total: i64,
    quota_total: i64,
    users: Vec<UserRow>,
}

/// Ringkasan seluruh user untuk admin. Otorisasi dicek **di server** memakai
/// role dari `/api/me`, bukan dari status tombol/tampilan halaman.
#[debug_handler]
async fn overview(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let caller = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let period = usages::Model::current_period();
    let access = user_profiles::Model::access(&ctx.db, caller.id, &period).await?;

    if access.role != "admin" {
        return json_response(
            StatusCode::FORBIDDEN,
            json!({ "error": "forbidden", "role": access.role }),
        );
    }

    let all_users = users::Entity::find().all(&ctx.db).await?;
    let all_profiles = user_profiles::Entity::find().all(&ctx.db).await?;
    let all_roles = roles::Entity::find().all(&ctx.db).await?;
    let all_plans = plans::Entity::find().all(&ctx.db).await?;
    let all_usages = usages::Entity::find().all(&ctx.db).await?;

    let role_names: HashMap<i64, String> = all_roles
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let plan_info: HashMap<i64, (String, i64)> = all_plans
        .into_iter()
        .map(|row| (row.id, (row.name, row.quota)))
        .collect();
    let profile_by_user: HashMap<i64, &user_profiles::Model> =
        all_profiles.iter().map(|row| (row.user_id, row)).collect();
    let usage_by_user: HashMap<i64, i64> = all_usages
        .into_iter()
        .filter(|row| row.period == period)
        .fold(HashMap::new(), |mut acc, row| {
            *acc.entry(row.user_id).or_insert(0) += row.sent;
            acc
        });

    let mut role_counts: HashMap<String, usize> = HashMap::new();
    let mut plan_counts: HashMap<String, usize> = HashMap::new();
    let mut rows = Vec::with_capacity(all_users.len());
    let mut sent_total = 0;
    let mut quota_total = 0;

    for user in all_users {
        let profile = profile_by_user.get(&user.id).copied();
        let role = profile
            .and_then(|p| role_names.get(&p.role_id))
            .cloned()
            .unwrap_or_else(|| "-".to_string());
        let (plan, quota) = profile
            .and_then(|p| plan_info.get(&p.plan_id))
            .cloned()
            .unwrap_or_else(|| ("-".to_string(), 0));
        let sent = usage_by_user.get(&user.id).copied().unwrap_or(0);

        *role_counts.entry(role.clone()).or_insert(0) += 1;
        *plan_counts.entry(plan.clone()).or_insert(0) += 1;
        sent_total += sent;
        quota_total += quota;

        rows.push(UserRow {
            pid: user.pid.to_string(),
            name: user.name,
            email: user.email,
            role,
            plan,
            sent,
            quota,
            joined: user.created_at.to_string(),
        });
    }

    let to_counts = |map: HashMap<String, usize>| -> Vec<CountRow> {
        let mut rows: Vec<CountRow> = map
            .into_iter()
            .map(|(name, total)| CountRow { name, total })
            .collect();
        rows.sort_by(|a, b| b.total.cmp(&a.total).then_with(|| a.name.cmp(&b.name)));
        rows
    };

    format::json(Overview {
        total_users: rows.len(),
        roles: to_counts(role_counts),
        plans: to_counts(plan_counts),
        period,
        sent_total,
        quota_total,
        users: rows,
    })
}

pub fn routes() -> Routes {
    Routes::new().add("/api/admin/overview", get(overview))
}
