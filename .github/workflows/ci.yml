# elcica-auth

Auth/SSO untuk ELCICA di `auth.elcica.xyz`.

## Status

Berjalan sebagai **Cloudflare Worker + D1**. Tidak perlu PC menyala, tidak perlu
server, tidak perlu kartu. Deploy otomatis dari `main` lewat GitHub Actions.

Versi Loco/Axum + SQLite lama ada di folder `src/controllers`, `src/models`,
`migration/`, dll. File-file itu **tidak ikut dikompilasi** dan disimpan hanya
sebagai referensi. Jangan diedit untuk perubahan produksi.

## Arsitektur

```text
Browser auth.elcica.xyz
        │
        ▼
Worker Rust (worker 0.9 + modul d1)
        │  binding AUTH_DB
        ▼
Cloudflare D1 (SQLite) — tabel users + usages
```

Cookie `elcica_session` memakai `Domain=.elcica.xyz`, `HttpOnly`, `Secure`,
`SameSite=Lax`, berlaku 7 hari, supaya bisa dipakai subdomain lain.

## Endpoint

```text
GET  /                       halaman depan
GET  /login                  form login
GET  /register               form daftar
GET  /admin                  halaman admin
GET  /health                 cek hidup -> JSON

POST /api/auth/register      daftar user
POST /api/auth/login         login -> cookie + JWT
POST /api/auth/logout        hapus cookie
GET  /api/me                 identitas, role, plan, kuota, periode
```

Email yang diterima: `@example.com` dan `@gmail.com` (sama seperti versi lama).

## Secret GitHub

Di **Settings → Secrets and variables → Actions**:

```text
CF_ACCOUNT_ID    sudah ada
CF_API_TOKEN     sudah ada
D1_DATABASE_ID   UUID dari workflow "D1 setup (satu kali)"
JWT_SECRET       base64, minimal 32 byte. Contoh: openssl rand -base64 32
```

Nilai secret tidak pernah disimpan di repo dan tidak dicetak di log.

## Deploy

1. Jalankan sekali: **Actions → D1 setup (satu kali) → Run workflow**.
   Salin UUID yang muncul ke secret `D1_DATABASE_ID`.
2. Push/merge ke `main`. Workflow **Deploy ELCICA Auth ke Cloudflare** akan:
   - mengisi `database_id` di `wrangler.toml`,
   - menjalankan `migrations/0001_init.sql` (idempotent),
   - build `worker-build --release`,
   - `wrangler deploy` dengan Custom Domain `auth.elcica.xyz`.

Cek manual:

```bash
curl -s https://auth.elcica.xyz/health
curl -s https://auth.elcica.xyz/login | head
```

## Catatan keamanan

Password disimpan sebagai `pbkdf2$<salt>$<hash>` (PBKDF2-HMAC-SHA256,
10.000 iterasi). Iterasi sengaja rendah karena Workers Free punya batas
**10 ms CPU per request**. Untuk produksi skala besar, naikkan iterasi setelah
pindah ke Workers Paid atau pindahkan verifikasi ke Durable Object.
