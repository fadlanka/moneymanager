# CLAUDE.md — Budgeting App (AI receipt scanner)

Konteks proyek untuk Claude Code. Baca file ini sebelum mengerjakan apa pun.

## Ringkasan

Aplikasi budgeting dan pencatatan pengeluaran pribadi untuk **Fadlan dan ~4 teman (total 5 user)**.
Fitur intinya: user **share foto struk** dari galeri/WhatsApp ke aplikasi, AI mengekstrak isinya
(merchant, tanggal, item, total, kategori), user mengoreksi di layar review, lalu tersimpan sebagai transaksi.

Proyek ini juga **portofolio AI engineering**. Nilai utamanya ada di pipeline ekstraksi struk dan
evaluasinya, bukan di CRUD. Selalu utamakan desain yang bisa diukur, bisa diganti provider-nya,
dan jujur soal kapan LLM boleh dipercaya.

## Komunikasi

- Jawab Fadlan dalam **Bahasa Indonesia**; istilah teknis boleh bahasa Inggris.
- Feedback langsung dan jujur. Kalau ada keputusan yang kurang tepat, bilang.
- Fadlan suka arsitektur **modular**: file terpisah per fungsi, entrypoint (`main.rs` / `main.dart`) hanya sebagai orkestrator/wiring.

## Stack (keputusan saat ini)

| Bagian | Pilihan | Alasan |
|---|---|---|
| Mobile app | **Flutter** (Android APK + web/PWA untuk iPhone) | Menerima share intent sudah matang (`receive_sharing_intent`); Fadlan sudah pakai Flutter |
| Backend API | **Rust — Axum + sqlx + tokio** | Ringan RAM (VPS kecil), cepat; kerjanya hanya HTTP + DB + panggil LLM |
| Database | **PostgreSQL** | Relasional, multi-user, transaksi uang |
| LLM ekstraksi | Provider-agnostic: **Gemini Flash** atau **Ollama (lokal)** lewat interface OpenAI-compatible | Bisa diganti lewat config, dan dibandingkan di evaluasi |
| Evaluasi | **Python** di folder `eval/` | Ekosistem analisis data/OCR lebih lengkap |
| Deploy | Docker Compose di VPS | Satu `docker compose up` |

Alternatif yang sempat dipertimbangkan: full Rust (Dioxus/Tauri v2) untuk UI. Ditunda karena menerima
share intent Android di ekosistem Rust belum matang (kemungkinan perlu tulis plugin Kotlin sendiri).
Jangan beralih ke sini tanpa persetujuan Fadlan.

## Struktur repo (usulan)

```
/
├── CLAUDE.md
├── docker-compose.yml
├── .env.example
├── backend/                 # Rust, Axum
│   ├── Cargo.toml
│   ├── migrations/          # sqlx migrations
│   └── src/
│       ├── main.rs          # wiring saja: config, router, state, spawn worker
│       ├── config.rs
│       ├── error.rs
│       ├── auth/            # login, JWT, middleware user_id
│       ├── routes/          # handler HTTP tipis per resource
│       ├── db/              # query sqlx per tabel
│       ├── extraction/
│       │   ├── mod.rs       # trait ReceiptExtractor
│       │   ├── gemini.rs
│       │   ├── ollama.rs    # OpenAI-compatible endpoint
│       │   ├── schema.rs    # struct hasil ekstraksi + JSON schema
│       │   └── validate.rs  # cek aritmatika, normalisasi angka
│       ├── queue/           # antrean job + rate limiter
│       └── budget/          # kalkulator deterministik (tanpa LLM)
├── app/                     # Flutter
│   └── lib/
│       ├── main.dart        # wiring saja
│       ├── theme/           # token desain (lihat bagian Desain)
│       ├── api/             # klien HTTP ke backend
│       ├── features/
│       │   ├── home/
│       │   ├── activity/
│       │   ├── scan/        # share intent, kamera, review, simpan
│       │   ├── budget/
│       │   ├── debts/
│       │   └── auth/
│       └── widgets/         # komponen bersama (card, pill, bar, tag)
└── eval/                    # Python
    ├── data/                # foto struk + ground truth (JANGAN commit foto asli ke repo publik)
    ├── run_eval.py
    └── metrics.py
```

## Alur utama: share struk → transaksi

1. User share gambar ke app (Android share sheet) **atau** foto dari kamera/galeri di layar Scan.
2. App upload gambar ke `POST /receipts` → backend simpan file, buat job `pending`, balas `receipt_id`.
3. Worker mengambil job dari antrean (rate-limited), memanggil extractor, menjalankan validasi, simpan hasil (`needs_review` / `failed`).
4. App polling `GET /receipts/{id}` (atau SSE) dan menampilkan **status loading** ("sedang membaca struk").
5. Layar **Review**: field dengan confidence rendah atau gagal validasi diberi highlight kuning.
6. User konfirmasi → `POST /transactions` dari hasil review. **Koreksi user disimpan** (diff vs hasil AI) untuk data evaluasi.

Dengan model lokal di CPU, satu struk bisa 30–60 detik. Proses selalu asinkron, jangan blok request HTTP.

## Kontrak ekstraksi

LLM wajib mengembalikan JSON terstruktur (pakai structured output / JSON schema bila provider mendukung).
Setiap field punya `confidence` 0–1.

```json
{
  "merchant": { "value": "Indomaret", "confidence": 0.97 },
  "date": { "value": "2026-10-15T19:05:00+07:00", "confidence": 0.9 },
  "items": [
    { "name": "Indomie Goreng", "qty": 5, "unit_price": 3500, "total": 17500, "confidence": 0.95 }
  ],
  "subtotal": { "value": 87400, "confidence": 0.8 },
  "tax": { "value": 0, "confidence": 0.6 },
  "discount": { "value": 0, "confidence": 0.6 },
  "total": { "value": 87400, "confidence": 0.98 },
  "category_suggestion": { "value": "shopping", "confidence": 0.7 },
  "currency": "IDR"
}
```

Aturan:
- **Uang selalu integer rupiah (`i64`)**, tidak pernah float. Di Dart pakai `int`.
- LLM hanya **membaca**, kode yang **menghitung**. Validasi di `extraction/validate.rs`:
  - `qty * unit_price == item.total`
  - jumlah item (± diskon, ± pajak) dibandingkan dengan `total`; kalau selisih → tandai `needs_review`
  - tanggal tidak di masa depan, tidak terlalu lampau
- Field yang gagal validasi → confidence diturunkan, item di-highlight di UI.
- Matikan / kecilkan thinking budget untuk ekstraksi (pengalaman sebelumnya: thinking token bikin respons terpotong).

### Kekhasan struk Indonesia (masukkan ke prompt + tes)

- Pemisah ribuan titik: `Rp 25.000` = 25000; koma bisa desimal (`25.000,00`).
- Jangan tertukar `TOTAL` dengan `TUNAI` / `BAYAR` (uang yang diberikan) atau `KEMBALI` (kembalian).
- PPN / pajak restoran / service charge, diskon member, "HEMAT".
- Struk thermal pudar, foto miring, struk panjang terpotong.
- Nama item disingkat (`INDOMIE GRG`).

## Budgeting & AI planner

- **Kalkulasi budget deterministik** di `backend/src/budget/`: alokasi per kategori, sisa per hari,
  strategi pelunasan hutang (avalanche / snowball), proyeksi jatuh tempo cicilan.
- LLM hanya: (1) mengubah input bebas user ("gaji 6 juta, cicilan laptop 800rb sampai Maret…") menjadi data
  terstruktur lewat **function calling**, (2) menjelaskan hasil kalkulator dalam bahasa manusia, (3) menerima revisi.
- Saran AI tidak pernah diterapkan otomatis: selalu lewat tombol konfirmasi ("Apply" / "Not now" di desain).

## Multi-user, privasi, keamanan

- 5 user. **Setiap query wajib difilter `user_id`** dari token. Tidak ada endpoint yang bisa membaca data user lain,
  kecuali tabel patungan (fase 2) yang memang dibagi antar peserta.
- Gemini **free tier** bisa memakai data untuk perbaikan model dan tidak untuk komersial. Data teman = data keuangan
  orang lain: provider default harus jelas di config, dan teman harus tahu/setuju. Opsi: paid tier atau Ollama lokal.
- Password di-hash (argon2). Secret hanya di `.env`, tidak pernah di-commit. HTTPS di depan backend.
- Backup PostgreSQL terjadwal. Foto struk disimpan di volume terpisah.
- Rate limit + antrean wajib ada walau volumenya kecil (~50 struk/hari total): batas yang sering kena adalah **per menit**, bukan per hari.

## Desain

Referensi desain (Claude Design, versi "Budgeting App Modern"):
https://claude.ai/artifact/2JQnuixxEqKRkX6ccTFh9v

Layar yang sudah ada: Home, Activity, Transaction detail, Scan (1 kamera → 2 review → 3 split & save),
Budget (Overview / Categories / Insights), Splits, New split, Tab bar. Ukuran acuan 390×844.

Token desain (implementasikan di `app/lib/theme/`):

```
bg #f3f4f8   card #ffffff   hair(border) #e5e7ef   track #eceef4
ink #0a0a0a  muted #5b5f6b
blue #0038ff (primary)  blue-dark #002bc4  blue-soft #e6ecff
red #ff1a1a  red-soft #ffe9e9  red-text #b30000
green #00c04b  green-soft #e0f7e9  green-text #00662a
yellow #ffd400  yellow-soft #fff6cc  yellow-text #5c4a00
font: Plus Jakarta Sans (google_fonts), angka pakai tabular figures
radius: card 20, button 16, icon button 14, pill 999
button min-height 52 (sm 40); touch target minimal 44
```

Warna kategori: Food & drinks = yellow, Shopping = red, Transport = green, Housing = blue, Subscriptions = ink.

**Celah desain yang perlu dibuat saat implementasi** (belum ada di artboard):
- Status loading / error ekstraksi, dan masuk langsung ke Review saat app dibuka dari share intent
- Layar hutang & cicilan
- Chat perencana budget (input kebutuhan)
- Login / onboarding / profil
- Form transaksi manual (tombol "Add" / "Type it" sudah ada)

## Fase

**Fase 1 — MVP (kerjakan ini dulu)**
1. Backend: skeleton Axum, config, migrasi (users, categories, transactions, receipts, debts, budgets), auth
2. Pipeline ekstraksi: trait extractor, provider Gemini + Ollama, validasi, antrean, endpoint receipts
3. Flutter: theme, Home, Activity, Scan (share intent + kamera + review), transaksi manual
4. Budget per kategori + hutang/cicilan + kalkulator deterministik

**Fase 2**
- Splits / patungan antar user (tabel bersama, status lunas)
- Push notification (FCM) untuk payment request dan pengingat
- Chat perencana budget dengan function calling

**Fase 3 — Evaluasi (bagian portofolio)**
- Dataset 50–100 foto struk Indonesia milik sendiri + label ground truth
- Akurasi per field (total, tanggal, merchant, item) untuk beberapa konfigurasi:
  Gemini Flash vs VLM lokal (Ollama) vs OCR (Tesseract) + LLM teks
- Catat latensi dan biaya per struk. Hasil yang belum ada tulis sebagai `...` di laporan.
- Saat menjalankan eval batch, beri jeda / rate limit (jangan ulangi masalah 429 seperti evaluasi RAGAS di skripsi).

## Keputusan yang sudah diambil

- **Platform**: Android = APK (jalur utama). Teman pengguna iPhone (iPhone 17 Air, iOS 26) memakai **build web Flutter sebagai PWA**
  ("Add to Home Screen") karena iOS tidak bisa install app di luar App Store tanpa akun Apple Developer.
  Konsekuensi: di PWA tidak ada share-to-app, upload lewat tombol kamera/galeri. iOS native = opsional (butuh Mac + TestFlight).
- **Tab kelima** di desain (Splits) diganti **Debts** untuk fase 1; Splits = fase 2.
- Kartu saran budget berlabel **"Suggestion"**, bukan "AI suggestion", karena angkanya dari kalkulator deterministik.
- **Push notification = fase 2**, pakai **FCM (gratis)**. Web push ke PWA iPhone jalan di iOS 16.4+ setelah Add to Home Screen.
  Prioritas: (1) struk selesai dibaca, (2) pengingat cicilan H-3/H-0, (3) budget kategori 85%/100%. Opsional: patungan, pengingat harian.
- Registrasi hanya dengan `INVITE_CODE`.
- Repo: https://github.com/fadlanka/moneymanager

## Keputusan yang masih terbuka (tanyakan ke Fadlan bila relevan)

- **Bahasa UI**: string masih bahasa Inggris, semua ada di `app/lib/l10n/strings.dart`.
- Provider ekstraksi default: Gemini (akurasi) atau Ollama (privasi). Teman harus tahu data struknya dikirim ke mana.

## Status MVP (fase 1)

- Backend selesai dan teruji: 45 unit test + 3 tes API (Postgres sungguhan), plus uji manual binary dengan server
  OpenAI-compatible palsu (JSON dalam ```fence, angka "Rp 26.000", tanggal "15/10/26", item math salah → ter-flag).
  Jalankan tes: `cd backend && DATABASE_URL=postgres://... cargo test`. Query sqlx offline ada di `backend/.sqlx` (`cargo sqlx prepare -- --all-targets` setelah ubah query).
- Flutter: kode lengkap tapi **belum pernah di-compile** (ditulis di lingkungan tanpa Flutter SDK). Langkah pertama di laptop:
  `cd app && flutter create . --org com.fadlanka --project-name moneymanager --platforms android,ios,web && flutter pub get && flutter analyze`, lalu perbaiki error yang muncul.
- Belum ada: push notification, Splits, chat planner LLM, evaluasi (`eval/` baru berisi rencana + query export koreksi).

## Lingkungan dev

- Laptop Windows (Acer Nitro V 15, ada GPU) → bisa menjalankan Ollama lokal untuk development & evaluasi.
- Pernah kena OOM di VPS CPU-only karena terinstal PyTorch versi GPU: di server jangan pasang paket GPU, siapkan swap,
  dan pertimbangkan model kecil kalau inference lokal dijalankan di VPS.

## Konvensi kode

- Rust: `cargo fmt` + `cargo clippy -- -D warnings`; error pakai `thiserror` di domain, `anyhow` hanya di main/bin.
  Handler tipis, logika di modul domain. Query pakai `sqlx::query!` (dicek saat compile).
- Flutter: `flutter analyze` bersih; satu folder per fitur; state management pilih satu dan konsisten (usulan: Riverpod).
- Tes: unit test untuk `validate.rs` dan `budget/` wajib (fixture struk dalam JSON). Jangan memanggil LLM sungguhan di unit test.
- Timezone default `Asia/Jakarta`. Format rupiah: `Rp 4.250.000`.
