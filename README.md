# Money Manager

A budgeting app for a small group of friends, built around one action:
**share a receipt photo, check what the AI read, save.**

- **Flutter** app (Android APK, web/PWA for iPhone)
- **Rust** API (Axum + sqlx + PostgreSQL)
- **Receipt extraction** with a vision LLM — Gemini or any OpenAI-compatible endpoint (Ollama) — behind one interface
- **Deterministic budget math**: the model reads receipts; code does every calculation

## How a receipt flows

```
phone ──share/scan──▶ POST /api/receipts ──▶ receipts (pending)
                                               │
                     worker (Postgres queue, SKIP LOCKED, rate limited, retries)
                                               │
                             extractor: Gemini | Ollama | mock
                                               │
                 validate.rs: Indonesian number formats, TOTAL vs TUNAI/KEMBALI,
                 qty×price, items vs total (± tax/discount), dates, confidence
                                               │
                            ready / needs_review + issues
                                               │
app polls GET /api/receipts/{id} ──▶ review screen (flagged fields highlighted)
                                               │
                 POST /api/transactions {receipt_id}  ──▶ stores the user's
                 corrections (diff vs AI) for evaluation
```

Why this shape:
- **Async**: a local model on CPU can take 30–60 s per receipt; HTTP never waits on the model.
- **Rate limited**: free LLM tiers fail on requests *per minute*; calls are spaced evenly.
- **The model only transcribes.** Line math, totals, and the "Rp 25.000 read as 25" class of errors are caught in code (`backend/src/extraction/validate.rs`).
- **Every correction is data.** What the user changed on the review screen is saved per field, per model and per prompt version (`eval/`).

## Run locally

Requirements: Rust (stable), PostgreSQL 16, Flutter.

```bash
# 1. Database
createdb moneymanager

# 2. Backend
cd backend
cat > .env <<'EOF'
DATABASE_URL=postgres://localhost/moneymanager
JWT_SECRET=dev-secret-that-is-at-least-32-characters
INVITE_CODE=teman
EXTRACTOR_PROVIDER=mock        # or gemini (+ GEMINI_API_KEY) / ollama
EOF
cargo run                      # runs migrations, listens on :8080

# 3. App (Android emulator)
cd ../app
flutter create . --org com.fadlanka --project-name moneymanager --platforms android,ios,web
flutter run --dart-define=API_BASE_URL=http://10.0.2.2:8080/api
```

`EXTRACTOR_PROVIDER=mock` returns a fixed Indomaret receipt, so the whole flow works without an API key.

### Tests

```bash
cd backend
DATABASE_URL=postgres://localhost/postgres cargo test   # unit + API tests (creates temp DBs)
cd ../app && flutter test
```

## Deploy (VPS)

```bash
cp .env.example .env          # fill in DOMAIN, POSTGRES_PASSWORD, JWT_SECRET, INVITE_CODE, GEMINI_API_KEY
(cd app && flutter build web --release --dart-define=API_BASE_URL=https://$DOMAIN/api)
docker compose up -d --build  # Postgres + API + Caddy (HTTPS, serves the PWA, proxies /api)
./deploy/backup.sh            # add to cron for nightly backups
```

Optional local model: `docker compose --profile ollama up -d` (needs ~8 GB RAM for a 7B vision model).

## API

All routes are under `/api` and, except `auth` and `health`, need `Authorization: Bearer <token>`.
Every query is scoped to the token's user.

| Method | Path | |
|---|---|---|
| POST | `/auth/register` | `{email, name, password, invite_code}` |
| POST | `/auth/login` | `{email, password}` → `{token, user}` |
| GET/PATCH | `/me` | name, `monthly_income` (used by the planner) |
| GET/POST | `/categories` | default categories are seeded on sign-up |
| POST | `/receipts` | multipart `image` → `202 {id}` |
| GET | `/receipts/{id}` | status, extraction, issues, suggested category |
| GET | `/receipts/{id}/image` | the stored photo |
| POST | `/receipts/{id}/retry` | requeue a failed receipt |
| GET/POST | `/transactions` | filters: `month=2026-10`, `kind`, `needs_check`, `q`, `category_id` |
| GET/PATCH/DELETE | `/transactions/{id}` | |
| GET | `/summary?month=` | totals, budget status, upcoming installments, suggestion, daily spend |
| GET/PUT | `/budgets` | per-category monthly limits (pre-filled from the last month) |
| POST | `/budgets/apply-suggestion` | apply the server-computed rebalance |
| GET/POST | `/debts`, PUT/DELETE `/debts/{id}` | installments and loans |
| POST | `/debts/{id}/payments` | lowers the balance, records an expense |
| GET | `/debts/plan?strategy=avalanche\|snowball&extra=` | payoff simulation |

## Repo

```
backend/   Rust API, migrations, sqlx offline data, tests
app/       Flutter app
deploy/    Caddyfile, backup script
eval/      extraction evaluation (phase 3)
CLAUDE.md  project context and decisions
```

## Roadmap

- Phase 2: push notifications (FCM: receipt ready, installment due, budget alerts), splits with friends, LLM budget planner via function calling
- Phase 3: labelled receipt benchmark across Gemini / local VLM / OCR + LLM
