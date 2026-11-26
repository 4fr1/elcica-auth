use crate::{
    data::settings::Settings,
    models::{usages, user_profiles, users},
};
use axum::{body::Body as AxumBody, response::Response, Json};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{header::CONTENT_TYPE, Method, Request, StatusCode, Uri};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use loco_rs::prelude::*;
use serde_json::{json, Value};

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

/// Teruskan pesan teks ke waxum, dengan pemeriksaan kuota bulanan.
///
/// Kuota dipotong **sebelum** request diteruskan. Jika waxum gagal, pesan
/// tidak terkirim tetapi kuota tetap terhitung — disengaja agar tidak ada
/// race condition saat banyak request bersamaan.
#[debug_handler]
async fn send_text(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(session_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Response> {
    if !session_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_session_id" }),
        );
    }

    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let period = usages::Model::current_period();
    let access = user_profiles::Model::access(&ctx.db, user.id, &period).await?;

    if access.sent >= access.quota {
        return json_response(
            StatusCode::TOO_MANY_REQUESTS,
            json!({
                "error": "quota_exceeded",
                "quota": access.quota,
                "sent": access.sent,
                "period": period,
            }),
        );
    }

    let sent = usages::Model::consume(&ctx.db, user.id, &period, access.quota).await?;

    let settings = Settings::from_context(&ctx)?;
    if settings.waxum.base_url.is_empty() || settings.waxum.token.is_empty() {
        return json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({ "error": "waxum_not_configured" }),
        );
    }

    let uri: Uri = format!(
        "{}/api/v1/sessions/{}/messages/text",
        settings.waxum.base_url.trim_end_matches('/'),
        session_id
    )
    .parse()
    .map_err(|e| Error::Message(format!("alamat waxum tidak valid: {e}")))?;

    let body = serde_json::to_vec(&payload)?;
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .header("authorization", format!("Bearer {}", settings.waxum.token))
        .body(Full::new(Bytes::from(body)))?;

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
        .to_bytes();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));

    tracing::info!(
        pid = user.pid.to_string(),
        sent,
        upstream = status.as_u16(),
        "wa proxy: pesan diteruskan"
    );

    json_response(status, value)
}

pub fn routes() -> Routes {
    Routes::new().add("/api/wa/{session_id}/messages/text", post(send_text))
}
