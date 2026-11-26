use loco_rs::{app::AppContext, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SsoSettings {
    /// Kosong = host-only cookie (cocok untuk localhost).
    /// Isi `.elcica.xyz` di produksi agar ikut ke semua subdomain.
    pub cookie_domain: String,
    pub cookie_secure: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WaxumSettings {
    /// Base URL internal waxum, mis. `http://localhost:3451`.
    pub base_url: String,
    /// Token server-to-server untuk waxum. Hanya disimpan di file lokal yang di-gitignore.
    pub token: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub sso: SsoSettings,
    pub waxum: WaxumSettings,
}

impl Settings {
    /// # Errors
    /// Gagal hanya jika blok `settings:` tidak bisa di-deserialize.
    pub fn from_context(ctx: &AppContext) -> Result<Self> {
        ctx.config.settings::<Self>()
    }
}
