use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use hmac::{Hmac, Mac};
use js_sys::Date;
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use wasm_bindgen::JsValue;
use worker::*;

const COOKIE_NAME: &str = "elcica_session";
const COOKIE_MAX_AGE: u64 = 604_800;
const PBKDF2_ROUNDS: u32 = 10_000;

const SHELL: &str = r#"<!doctype html>
<html lang="id">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>__TITLE__ | ELCICA</title>
<style>
:root{color-scheme:dark}
*{box-sizing:border-box}
body{margin:0;min-height:100vh;background:#080d0b;color:#d7e6dc;font:16px/1.6 system-ui,sans-serif;display:flex;flex-direction:column}
header{padding:18px 28px;border-bottom:1px solid #1d3227}
header a{color:#7ee0a3;text-decoration:none;font-weight:700;letter-spacing:.08em}
main{flex:1;display:grid;place-items:center;padding:32px 20px}
.card{width:min(420px,100%);background:#0d1812;border:1px solid #1d3227;border-radius:14px;padding:28px}
h1{margin:0 0 6px;color:#7ee0a3;font-size:1.5rem}
p.sub{margin:0 0 22px;color:#8fa89a;font-size:.9rem}
label{display:block;margin:14px 0 6px;font-size:.85rem;color:#8fa89a}
input{width:100%;padding:11px 12px;border-radius:8px;border:1px solid #294535;background:#07110b;color:#e6f3ea}
button{width:100%;margin-top:22px;padding:12px;border:0;border-radius:8px;background:#7ee0a3;color:#07110b;font-weight:700;cursor:pointer}
a.alt{display:block;margin-top:16px;text-align:center;color:#8fa89a;font-size:.85rem}
code{color:#7ee0a3}
</style>
</head>
<body>
<header><a href="/">ELCICA AUTH</a></header>
<main><div class="card">__BODY__</div></main>
</body>
</html>"#;

const INDEX_BODY: &str = r#"
<h1>ELCICA Auth</h1>
<p class="sub">Akses terpusat untuk layanan ELCICA.</p>
<p><a class="alt" href="/login">Masuk</a></p>
<p><a class="alt" href="/register">Daftar</a></p>
<p><a class="alt" href="/admin">Halaman admin</a></p>
"#;

const LOGIN_BODY: &str = r#"
<h1>Masuk</h1>
<p class="sub">Cookie <code>elcica_session</code> berlaku untuk seluruh <code>.elcica.xyz</code>.</p>
<form id="f">
<label for="email">Email</label>
<input id="email" type="email" autocomplete="username" required>
<label for="password">Kata sandi</label>
<input id="password" type="password" autocomplete="current-password" required>
<button type="submit">Masuk</button>
</form>
<p id="msg" class="sub"></p>
<p><a class="alt" href="/register">Belum punya akun? Daftar</a></p>
<script>
document.getElementById('f').addEventListener('submit', async function (e) {
  e.preventDefault();
  const msg = document.getElementById('msg');
  msg.textContent = 'Memeriksa...';
  const res = await fetch('/api/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      email: document.getElementById('email').value,
      password: document.getElementById('password').value
    })
  });
  if (res.ok) { location.href = '/admin'; return; }
  msg.textContent = 'Login gagal. Periksa email dan kata sandi.';
});
</script>
"#;

const REGISTER_BODY: &str = r#"
<h1>Daftar</h1>
<p class="sub">Email yang diterima: <code>@example.com</code> dan <code>@gmail.com</code>.</p>
<form id="f">
<label for="name">Nama</label>
<input id="name" required>
<label for="email">Email</label>
<input id="email" type="email" required>
<label for="password">Kata sandi</label>
<input id="password" type="password" minlength="8" required>
<button type="submit">Daftar</button>
</form>
<p id="msg" class="sub"></p>
<p><a class="alt" href="/login">Sudah punya akun? Masuk</a></p>
<script>
document.getElementById('f').addEventListener('submit', async function (e) {
  e.preventDefault();
  const msg = document.getElementById('msg');
  msg.textContent = 'Menyimpan...';
  const res = await fetch('/api/auth/register', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      name: document.getElementById('name').value,
      email: document.getElementById('email').value,
      password: document.getElementById('password').value
    })
  });
  if (res.ok) { location.href = '/login'; return; }
  msg.textContent = 'Pendaftaran gagal. Email mungkin sudah dipakai.';
});
</script>
"#;

const ADMIN_BODY: &str = r#"
<h1>Admin</h1>
<p class="sub">Identitas dan role diputuskan server lewat <code>/api/me</code>.</p>
<pre id="me">Memuat identitas...</pre>
<p><a class="alt" href="/api/auth/logout">Keluar</a></p>
<script>
(async function () {
  const el = document.getElementById('me');
  const res = await fetch('/api/me');
  if (!res.ok) { location.href = '/login'; return; }
  el.textContent = JSON.stringify(await res.json(), null, 2);
})();
</script>
"#;

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Serialize, Clone)]
struct UserRow {
    pid: String,
    email: String,
    name: String,
    password_hash: String,
    role: String,
    plan: String,
}

#[derive(Serialize)]
struct MeResponse {
    pid: String,
    name: String,
    email: String,
    role: String,
    plan: String,
    quota: i64,
    sent: i64,
    period: String,
}

fn err(msg: impl Into<String>) -> Error {
    Error::RustError(msg.into())
}

fn now_secs() -> u64 {
    (Date::now() / 1000.0) as u64
}

fn period_now() -> String {
    let d = Date::new_0();
    format!("{:04}-{:02}", d.get_utc_full_year(), d.get_utc_month() + 1)
}

fn random_bytes(len: usize) -> Vec<u8> {
    let arr = js_sys::Uint8Array::new_with_length(len as u32);
    let global = js_sys::global();
    let crypto = js_sys::Reflect::get(&global, &JsValue::from_str("crypto"))
        .expect("crypto global tidak tersedia");
    let fun = js_sys::Reflect::get(&crypto, &JsValue::from_str("getRandomValues"))
        .expect("getRandomValues tidak tersedia");
    let fun: js_sys::Function = fun.into();
    fun.call1(&crypto, &arr).expect("getRandomValues gagal");
    arr.to_vec()
}

fn hash_password(password: &str, salt: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ROUNDS, &mut out);
    out
}

fn make_hash(password: &str) -> String {
    let salt = random_bytes(16);
    format!(
        "pbkdf2${}${}",
        STANDARD.encode(&salt),
        STANDARD.encode(hash_password(password, &salt))
    )
}

fn verify_password(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 3 || parts[0] != "pbkdf2" {
        return false;
    }
    let salt = match STANDARD.decode(parts[1]) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let expected = match STANDARD.decode(parts[2]) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let actual = hash_password(password, &salt);
    if actual.len() != expected.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in actual.iter().zip(expected.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

fn secret_bytes(env: &Env) -> Result<Vec<u8>> {
    let raw = env.secret("JWT_SECRET")?.to_string();
    STANDARD
        .decode(raw.trim())
        .map_err(|e| err(format!("JWT_SECRET bukan base64: {}", e)))
}

fn sign_jwt(secret: &[u8], pid: &str) -> Result<String> {
    let header = serde_json::json!({ "alg": "HS256", "typ": "JWT" });
    let payload = serde_json::json!({
        "sub": pid,
        "iat": now_secs(),
        "exp": now_secs() + COOKIE_MAX_AGE
    });
    let h = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(|e| err(e.to_string()))?);
    let p = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).map_err(|e| err(e.to_string()))?);
    let input = format!("{}.{}", h, p);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|e| err(e.to_string()))?;
    mac.update(input.as_bytes());
    let sig = mac.finalize().into_bytes();
    Ok(format!("{}.{}", input, URL_SAFE_NO_PAD.encode(sig)))
}

fn subject_from_jwt(secret: &[u8], token: &str) -> Option<String> {
    let mut it = token.split('.');
    let h = it.next()?;
    let p = it.next()?;
    let s = it.next()?;
    if it.next().is_some() {
        return None;
    }
    let sig = URL_SAFE_NO_PAD.decode(s).ok()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).ok()?;
    mac.update(format!("{}.{}", h, p).as_bytes());
    let expected = mac.finalize().into_bytes();
    if expected.as_slice() != sig.as_slice() {
        return None;
    }
    let payload_bytes = URL_SAFE_NO_PAD.decode(p).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;
    if payload.get("exp")?.as_u64()? < now_secs() {
        return None;
    }
    Some(payload.get("sub")?.as_str()?.to_string())
}

fn cookie_value(headers: &Headers, name: &str) -> Option<String> {
    let raw = headers.get("Cookie").ok()??;
    raw.split(';').find_map(|part| {
        let mut kv = part.trim().splitn(2, '=');
        if kv.next()? == name {
            Some(kv.next()?.to_string())
        } else {
            None
        }
    })
}

fn session_cookie(token: &str) -> String {
    format!(
        "{}={}; Path=/; Domain=.elcica.xyz; Max-Age={}; HttpOnly; Secure; SameSite=Lax",
        COOKIE_NAME, token, COOKIE_MAX_AGE
    )
}

fn clear_cookie() -> String {
    format!(
        "{}=; Path=/; Domain=.elcica.xyz; Max-Age=0; HttpOnly; Secure; SameSite=Lax",
        COOKIE_NAME
    )
}

fn with_cookie(mut res: Response, value: String) -> Result<Response> {
    res.headers_mut().set("Set-Cookie", &value)?;
    res.headers_mut().set("Cache-Control", "no-store")?;
    Ok(res)
}

fn page(title: &str, body: &str) -> Result<Response> {
    Response::from_html(SHELL.replace("__TITLE__", title).replace("__BODY__", body))
}

fn email_allowed(email: &str) -> bool {
    let lower = email.to_lowercase();
    lower.ends_with("@example.com") || lower.ends_with("@gmail.com")
}

async fn find_user(db: &D1Database, email: &str) -> Result<Option<UserRow>> {
    db.prepare("SELECT pid, email, name, password_hash, role, plan FROM users WHERE email = ?")
        .bind(&[JsValue::from_str(email)])?
        .first::<UserRow>(None)
        .await
}

async fn handle_login(mut req: Request, env: &Env) -> Result<Response> {
    let body: Credentials = req.json().await?;
    if !email_allowed(&body.email) {
        return Response::from_json(&serde_json::json!({ "error": "email tidak diizinkan" }))
            .map(|r| r.with_status(400));
    }
    let db = env.d1("AUTH_DB")?;
    let Some(user) = find_user(&db, &body.email).await? else {
        return Response::from_json(&serde_json::json!({ "error": "kredensial tidak valid" }))
            .map(|r| r.with_status(401));
    };
    if !verify_password(&body.password, &user.password_hash) {
        return Response::from_json(&serde_json::json!({ "error": "kredensial tidak valid" }))
            .map(|r| r.with_status(401));
    }
    let secret = secret_bytes(env)?;
    let token = sign_jwt(&secret, &user.pid)?;
    let res = Response::from_json(&serde_json::json!({
        "token": token,
        "pid": user.pid,
        "name": user.name,
        "email": user.email
    }))?;
    with_cookie(res, session_cookie(&token))
}

async fn handle_register(mut req: Request, env: &Env) -> Result<Response> {
    let body: Credentials = req.json().await?;
    if !email_allowed(&body.email) || body.password.len() < 8 || body.name.is_empty() {
        return Response::from_json(&serde_json::json!({ "error": "data tidak valid" }))
            .map(|r| r.with_status(400));
    }
    let db = env.d1("AUTH_DB")?;
    if find_user(&db, &body.email).await?.is_some() {
        return Response::from_json(&serde_json::json!({ "error": "email sudah dipakai" }))
            .map(|r| r.with_status(409));
    }
    let pid = URL_SAFE_NO_PAD.encode(random_bytes(16));
    let hash = make_hash(&body.password);
    db.prepare(
        "INSERT INTO users (pid, email, name, password_hash, role, plan, created_at) VALUES (?, ?, ?, ?, 'user', 'free', ?)",
    )
    .bind(&[
        JsValue::from_str(&pid),
        JsValue::from_str(&body.email.to_lowercase()),
        JsValue::from_str(&body.name),
        JsValue::from_str(&hash),
        JsValue::from_f64(now_secs() as f64),
    ])?
    .run()
    .await?;
    db.prepare("INSERT OR IGNORE INTO usages (user_pid, period, sent) VALUES (?, ?, 0)")
        .bind(&[JsValue::from_str(&pid), JsValue::from_str(&period_now())])?
        .run()
        .await?;
    Response::from_json(&serde_json::json!({ "pid": pid }))
}

async fn handle_me(req: &Request, env: &Env) -> Result<Response> {
    let secret = secret_bytes(env)?;
    let Some(token) = cookie_value(req.headers(), COOKIE_NAME) else {
        return Response::from_json(&serde_json::json!({ "error": "belum login" }))
            .map(|r| r.with_status(401));
    };
    let Some(pid) = subject_from_jwt(&secret, &token) else {
        return Response::from_json(&serde_json::json!({ "error": "sesi tidak valid" }))
            .map(|r| r.with_status(401));
    };
    let db = env.d1("AUTH_DB")?;
    let user: Option<UserRow> = db
        .prepare("SELECT pid, email, name, password_hash, role, plan FROM users WHERE pid = ?")
        .bind(&[JsValue::from_str(&pid)])?
        .first::<UserRow>(None)
        .await?;
    let Some(user) = user else {
        return Response::from_json(&serde_json::json!({ "error": "pengguna tidak ditemukan" }))
            .map(|r| r.with_status(401));
    };
    let period = period_now();
    let sent: Option<i64> = db
        .prepare("SELECT sent FROM usages WHERE user_pid = ? AND period = ?")
        .bind(&[JsValue::from_str(&pid), JsValue::from_str(&period)])?
        .first::<i64>(Some("sent"))
        .await?;
    let quota = if user.plan == "paid" { 5000 } else { 100 };
    Response::from_json(&MeResponse {
        pid: user.pid,
        name: user.name,
        email: user.email,
        role: user.role,
        plan: user.plan,
        quota,
        sent: sent.unwrap_or(0),
        period,
    })
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let path = req.path();
    let method = req.method();

    match (method, path.as_str()) {
        (Method::Get, "/") => page("ELCICA", INDEX_BODY),
        (Method::Get, "/login") => page("Masuk", LOGIN_BODY),
        (Method::Get, "/register") => page("Daftar", REGISTER_BODY),
        (Method::Get, "/admin") => page("Admin", ADMIN_BODY),
        (Method::Get, "/favicon.ico") => Response::empty(),

        (Method::Get, "/health") => Response::from_json(&serde_json::json!({
            "app": "elcica-auth",
            "status": "ok",
            "version": "0.2.0-worker"
        })),

        (Method::Post, "/api/auth/login") => handle_login(req, &env).await,
        (Method::Post, "/api/auth/register") => handle_register(req, &env).await,
        (Method::Get, "/api/me") => handle_me(&req, &env).await,

        (Method::Post, "/api/auth/logout") => {
            with_cookie(Response::from_json(&serde_json::json!({ "ok": true }))?, clear_cookie())
        }

        _ => Response::error("Not Found", 404),
    }
}
