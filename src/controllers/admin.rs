use crate::{
    data::settings::Settings,
    models::{plans, roles, usages, user_profiles, users},
};
use axum::{body::Body as AxumBody, extract::Path, response::Response};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{header::CONTENT_TYPE, Method, Request, StatusCode, Uri};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use loco_rs::prelude::*;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

type ReqBody = Full<Bytes>;

fn client() -> Client<HttpConnector, ReqBody> {
    Client::builder(TokioExecutor::new()).build_http()
}

fn json_response(status: StatusCode, value: Value) -> Result<Response> {
    let bytes = serde_json::to_vec(&value)?;
    Ok(Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(AxumBody::from(bytes))?)
}

async fn is_admin(auth: &auth::JWT, ctx: &AppContext) -> Result<bool> {
    let caller = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let period = usages::Model::current_period();
    let access = user_profiles::Model::access(&ctx.db, caller.id, &period).await?;
    Ok(access.role == "admin")
}

fn forbidden() -> Result<Response> {
    json_response(StatusCode::FORBIDDEN, json!({ "error": "forbidden" }))
}

/// Panggil waxum dari sisi server. Token tidak pernah dikirim ke browser.
/// `cookie = true` dipakai untuk rute console `/qr-svg/{id}` yang mengembalikan SVG.
async fn waxum_call(
    ctx: &AppContext,
    method: Method,
    path: &str,
    body: Option<Value>,
    cookie: bool,
) -> Result<(StatusCode, Vec<u8>)> {
    let settings = Settings::from_context(ctx)?;
    if settings.waxum.base_url.is_empty() || settings.waxum.token.is_empty() {
        return Err(Error::Message("waxum_not_configured".to_string()));
    }

    let uri: Uri = format!("{}{}", settings.waxum.base_url.trim_end_matches('/'), path)
        .parse()
        .map_err(|e| Error::Message(format!("alamat waxum tidak valid: {e}")))?;

    let mut builder = Request::builder().method(method).uri(uri);
    if cookie {
        builder = builder.header("cookie", format!("waxum_console={}", settings.waxum.token));
    } else {
        builder = builder.header("authorization", format!("Bearer {}", settings.waxum.token));
    }

    let request = if let Some(payload) = body {
        builder
            .header(CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(serde_json::to_vec(&payload)?)))?
    } else {
        builder.body(Full::new(Bytes::new()))?
    };

    let upstream = client()
        .request(request)
        .await
        .map_err(|e| Error::Message(format!("waxum tidak bisa dihubungi: {e}")))?;
    let status = upstream.status();
    let bytes = upstream
        .into_body()
        .collect()
        .await
        .map_err(|e| Error::Message(format!("respons waxum gagal dibaca: {e}")))?
        .to_bytes()
        .to_vec();
    Ok((status, bytes))
}

fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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
    if !is_admin(&auth, &ctx).await? {
        return forbidden();
    }
    let period = usages::Model::current_period();

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

/// Daftar sesi WhatsApp dari waxum: id, nama, nomor, dan status.
#[debug_handler]
async fn wa_sessions(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    if !is_admin(&auth, &ctx).await? {
        return forbidden();
    }
    let (status, bytes) = waxum_call(&ctx, Method::GET, "/api/v1/sessions", None, false).await?;
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
    json_response(status, value)
}

/// Buat sesi baru. waxum langsung memulai pairing QR saat sesi dibuat.
#[debug_handler]
async fn wa_create(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    if !is_admin(&auth, &ctx).await? {
        return forbidden();
    }
    let id = Uuid::new_v4().to_string();
    let payload = json!({
        "id": id,
        "name": format!("ELCICA-{}", &id[..8]),
    });
    let (status, bytes) =
        waxum_call(&ctx, Method::POST, "/api/v1/sessions", Some(payload), false).await?;
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
    json_response(status, value)
}

/// SVG QR dari console waxum, diteruskan ke browser admin tanpa membocorkan token.
#[debug_handler]
async fn wa_qr(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(session_id): Path<String>,
) -> Result<Response> {
    if !is_admin(&auth, &ctx).await? {
        return forbidden();
    }
    if !valid_session_id(&session_id) {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_session_id" }),
        );
    }
    let path = format!("/qr-svg/{session_id}");
    let (status, bytes) = waxum_call(&ctx, Method::GET, &path, None, true).await?;
    Ok(Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "image/svg+xml")
        .body(AxumBody::from(bytes))?)
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/api/admin/overview", get(overview))
        .add("/api/admin/wa/sessions", get(wa_sessions))
        .add("/api/admin/wa/sessions/create", post(wa_create))
        .add("/api/admin/wa/sessions/{session_id}/qr.svg", get(wa_qr))
}
