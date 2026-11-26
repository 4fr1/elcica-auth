use loco_rs::prelude::*;

/// Serves the server-side rendered home page at `/`.
///
/// Without this the server-side starter had nothing at the root: it shipped
/// `assets/views/home/hello.html` and a test that rendered it, but no route
/// pointed at the template, so choosing `--assets serverside` and running the
/// app gave a 404 on the first page anyone visits.
///
/// It is also the only place a generated app demonstrates
/// `format::render().view(..)`. The controller takes the `ViewRenderer` trait
/// through the `ViewEngine` extractor rather than touching Tera directly, so
/// swapping the engine later leaves this call site compiling untouched.
#[debug_handler]
async fn index(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "home/index.html", data!({ "title": "ELCICA" }))
}

/// Halaman admin ringan. Tidak memercayakan role dari sisi klien untuk
/// otorisasi: halaman ini hanya menampilkan UI, sedangkan `/api/me` yang
/// memutuskan identitas/role dan tetap dilindungi JWT di server.
#[debug_handler]
async fn admin(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "home/admin.html", data!({ "title": "Admin" }))
}

/// Form login ringan. POST-nya tetap ke endpoint Loco `/api/auth/login`,
/// sehingga cookie `elcica_session` dibuat oleh server yang sama.
#[debug_handler]
async fn login(ViewEngine(v): ViewEngine<TeraView>) -> Result<Response> {
    format::render().view(&v, "home/login.html", data!({ "title": "Login" }))
}

pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
        .add("/admin", get(admin))
        .add("/login", get(login))
}
