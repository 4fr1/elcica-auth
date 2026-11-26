use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::controller::views::{engines, ViewRenderer};

/// Renders the shipped Tera view through the same view engine the app builds at
/// boot, including the i18n `t()` function the template calls.
///
/// This covers the whole server-side rendering path end to end: registering a
/// custom function, loading templates that use it, and resolving locales. The
/// template engine validates function references when templates are loaded, so
/// a registration-ordering mistake fails here rather than at runtime.
#[test]
fn renders_home_view_with_i18n() {
    let loader = std::sync::Arc::new(
        ArcLoader::builder("assets/i18n", unic_langid::langid!("en-US"))
            .shared_resources(Some(&["assets/shared.ftl".into()]))
            .customize(|bundle| bundle.set_use_isolating(false))
            .build()
            .expect("locales should load"),
    );

    let view = engines::TeraView::build_with_post_process(move |tera| {
        tera.register_function("t", FluentLoader::new(loader.clone()));
        Ok(())
    })
    .expect("view engine should build");

    let rendered = view
        .render("home/index.html", serde_json::json!({ "title": "ELCICA" }))
        .expect("home view should render");

    assert!(
        rendered.contains("ELCICA"),
        "expected the ELCICA landing to render, got: {rendered}"
    );

    let admin = view
        .render("home/admin.html", serde_json::json!({ "title": "Admin" }))
        .expect("admin view should render");

    assert!(admin.contains("ADMIN CONSOLE"), "admin heading missing");
    assert!(admin.contains("/api/me"), "admin page must check /api/me");
    assert!(admin.contains("/login"), "admin page must link to login");
    assert!(
        admin.contains("/api/auth/logout"),
        "admin page must offer logout"
    );
    assert!(
        admin.contains("/api/admin/overview"),
        "admin page must load the overview endpoint"
    );
    assert!(
        admin.contains("/api/admin/wa/sessions"),
        "admin page must load WhatsApp sessions"
    );

    let login = view
        .render("home/login.html", serde_json::json!({ "title": "Login" }))
        .expect("login view should render");

    assert!(login.contains("ELCICA LOGIN"), "login heading missing");
    assert!(
        login.contains("/api/auth/login"),
        "login must post to Loco auth"
    );
    assert!(login.contains("/register"), "login must link to register");

    let register = view
        .render(
            "home/register.html",
            serde_json::json!({ "title": "Daftar" }),
        )
        .expect("register view should render");

    assert!(
        register.contains("DAFTAR ELCICA"),
        "register heading missing"
    );
    assert!(
        register.contains("/api/auth/register"),
        "register must post to Loco auth"
    );
}
