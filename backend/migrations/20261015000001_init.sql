-- Initial schema. All money columns are BIGINT rupiah (never float).

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    password_hash   TEXT NOT NULL,
    monthly_income  BIGINT NOT NULL DEFAULT 0 CHECK (monthly_income >= 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE categories (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    slug        TEXT NOT NULL,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
    color       TEXT NOT NULL,
    -- Flexible categories may be cut when the planner rebalances a month.
    flexible    BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, slug)
);

CREATE TABLE receipts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status            TEXT NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('pending', 'processing', 'ready', 'needs_review', 'failed', 'confirmed')),
    image_path        TEXT NOT NULL,
    mime_type         TEXT NOT NULL,
    provider          TEXT,
    model             TEXT,
    prompt_version    TEXT,
    -- Validated extraction (the exact AI output after code-side checks).
    extraction        JSONB,
    issues            JSONB NOT NULL DEFAULT '[]'::jsonb,
    error             TEXT,
    attempts          INT NOT NULL DEFAULT 0,
    next_attempt_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    latency_ms        INT,
    input_tokens      INT,
    output_tokens     INT,
    -- What the user actually saved, and which fields differ from the AI output.
    -- This is the raw material for evaluation.
    confirmed         JSONB,
    corrected_fields  TEXT[],
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    processed_at      TIMESTAMPTZ
);
CREATE INDEX receipts_queue_idx ON receipts (next_attempt_at) WHERE status = 'pending';
CREATE INDEX receipts_user_idx ON receipts (user_id, created_at DESC);

CREATE TABLE transactions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category_id  UUID REFERENCES categories(id) ON DELETE SET NULL,
    kind         TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
    amount       BIGINT NOT NULL CHECK (amount > 0),
    merchant     TEXT NOT NULL,
    note         TEXT,
    occurred_at  TIMESTAMPTZ NOT NULL,
    source       TEXT NOT NULL CHECK (source IN ('manual', 'scan', 'debt_payment')),
    receipt_id   UUID REFERENCES receipts(id) ON DELETE SET NULL,
    items        JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- True when the transaction came from a scan with open validation issues.
    needs_check  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX transactions_user_time_idx ON transactions (user_id, occurred_at DESC);
CREATE UNIQUE INDEX transactions_receipt_uidx ON transactions (receipt_id) WHERE receipt_id IS NOT NULL;

CREATE TABLE budgets (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category_id   UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    month         DATE NOT NULL CHECK (EXTRACT(DAY FROM month) = 1),
    limit_amount  BIGINT NOT NULL CHECK (limit_amount >= 0),
    UNIQUE (user_id, category_id, month)
);

CREATE TABLE debts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    kind              TEXT NOT NULL CHECK (kind IN ('installment', 'loan')),
    creditor          TEXT,
    principal         BIGINT NOT NULL CHECK (principal > 0),
    balance           BIGINT NOT NULL CHECK (balance >= 0),
    monthly_payment   BIGINT NOT NULL DEFAULT 0 CHECK (monthly_payment >= 0),
    -- Annual interest rate in basis points: 1250 = 12.50% per year.
    interest_rate_bp  INT NOT NULL DEFAULT 0 CHECK (interest_rate_bp >= 0),
    due_day           INT CHECK (due_day BETWEEN 1 AND 31),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX debts_user_idx ON debts (user_id);

CREATE TABLE debt_payments (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    debt_id         UUID NOT NULL REFERENCES debts(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount          BIGINT NOT NULL CHECK (amount > 0),
    paid_at         TIMESTAMPTZ NOT NULL,
    transaction_id  UUID REFERENCES transactions(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX debt_payments_debt_idx ON debt_payments (debt_id, paid_at DESC);
