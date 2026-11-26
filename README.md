# elcica-auth

Backend auth/SSO Loco untuk ELCICA: user, role, plan, kuota bulanan, landing
`elcica.xyz`, dan proxy pesan WhatsApp ke waxum.

## Status

Masih **lokal**. Cloudflare belum diubah; `elcica.xyz` dan `wa.elcica.xyz`
belum diarahkan ke sini.

## Endpoint utama

```text
GET  /                              landing ELCICA
POST /api/auth/register
POST /api/auth/login                → cookie elcica_session + token
GET  /api/me                        → role, plan, quota, sent, period
POST /api/auth/logout
POST /api/wa/{session_id}/messages/text   → cek kuota, lalu forward ke waxum
```

Kuota dihitung per bulan (`YYYY-MM`). Plan seed: `free` 100 pesan, `paid` 5000.

## Jalankan lokal

Butuh XAMPP/MySQL? **Tidak** — project ini memakai SQLite.

```powershell
cargo loco db migrate
cargo loco start
```

Server berjalan di `http://localhost:5150`. Waxum lokal di `http://localhost:3451`.

## Konfigurasi sensitif

File `config/development.local.yaml` **tidak ikut committed** (sudah ada di
`.gitignore`). Isinya secret lokal, termasuk `JWT_SECRET` yang **wajib base64**.

Untuk produksi, jangan taruh nilai di repo. Simpan lewat GitHub GUI:

```text
Settings → Secrets and variables → Actions
```

Variabel produksi yang dipakai (nilainya tidak pernah disimpan di repo):

```text
DATABASE_URL      JWT_SECRET (wajib base64)   HOST
WAXUM_BASE_URL    WAXUM_TOKEN
MAILER_ENABLE     MAILER_HOST / MAILER_USER / MAILER_PASSWORD
PORT / BINDING    (opsional, default 5150 / 0.0.0.0)
```

Mailer default-nya mati di produksi; email gagal hanya jadi warning dan tidak
menggagalkan register/login. Token Cloudflare masuk GUI, bukan file repo.

## Workflow

`.github/workflows/ci.yml` jalan otomatis saat push/PR ke `main` atau `dev`:

```text
cargo fmt --check → cargo clippy -D warnings → cargo test → cargo build --release
```

Hasilnya terlihat di tab **Actions** di GitHub. Ikon merah di commit/PR berarti
ada langkah yang gagal, lengkap dengan log-nya.

## Catatan

- Entity di `src/models/_entities/` di-generate dari database. Jangan edit manual.
- `backup-products-image.txt` dan script PHP di workspace lain tidak berhubungan
  dengan project ini.
- Session WhatsApp harus tersambung dulu di waxum sebelum proxy bisa mengirim
  pesan nyata.
