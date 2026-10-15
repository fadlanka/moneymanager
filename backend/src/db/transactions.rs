use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Transaction {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub kind: String,
    pub amount: i64,
    pub merchant: String,
    pub note: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub source: String,
    pub receipt_id: Option<Uuid>,
    pub items: Value,
    pub needs_check: bool,
    pub created_at: DateTime<Utc>,
}

pub struct NewTransaction<'a> {
    pub user_id: Uuid,
    pub category_id: Option<Uuid>,
    pub kind: &'a str,
    pub amount: i64,
    pub merchant: &'a str,
    pub note: Option<&'a str>,
    pub occurred_at: DateTime<Utc>,
    pub source: &'a str,
    pub receipt_id: Option<Uuid>,
    pub items: Value,
    pub needs_check: bool,
}

#[derive(Debug, Default)]
pub struct ListFilter {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub category_id: Option<Uuid>,
    pub kind: Option<String>,
    pub needs_check: Option<bool>,
    pub search: Option<String>,
    pub limit: i64,
}

pub async fn insert(conn: &mut PgConnection, t: NewTransaction<'_>) -> sqlx::Result<Transaction> {
    sqlx::query_as!(
        Transaction,
        r#"INSERT INTO transactions
             (user_id, category_id, kind, amount, merchant, note, occurred_at, source, receipt_id, items, needs_check)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, category_id, kind, amount, merchant, note, occurred_at, source,
                     receipt_id, items, needs_check, created_at"#,
        t.user_id,
        t.category_id,
        t.kind,
        t.amount,
        t.merchant,
        t.note,
        t.occurred_at,
        t.source,
        t.receipt_id,
        t.items,
        t.needs_check
    )
    .fetch_one(conn)
    .await
}

pub async fn list(db: &PgPool, user_id: Uuid, f: &ListFilter) -> sqlx::Result<Vec<Transaction>> {
    let search = f
        .search
        .as_ref()
        .map(|s| format!("%{}%", s.replace('%', "\\%").replace('_', "\\_")));
    sqlx::query_as!(
        Transaction,
        r#"SELECT t.id, t.category_id, t.kind, t.amount, t.merchant, t.note, t.occurred_at, t.source,
                  t.receipt_id, t.items, t.needs_check, t.created_at
           FROM transactions t
           LEFT JOIN categories c ON c.id = t.category_id
           WHERE t.user_id = $1
             AND ($2::timestamptz IS NULL OR t.occurred_at >= $2)
             AND ($3::timestamptz IS NULL OR t.occurred_at < $3)
             AND ($4::uuid IS NULL OR t.category_id = $4)
             AND ($5::text IS NULL OR t.kind = $5)
             AND ($6::bool IS NULL OR t.needs_check = $6)
             AND ($7::text IS NULL OR t.merchant ILIKE $7 OR t.note ILIKE $7 OR c.name ILIKE $7)
           ORDER BY t.occurred_at DESC, t.created_at DESC
           LIMIT $8"#,
        user_id,
        f.from,
        f.to,
        f.category_id,
        f.kind,
        f.needs_check,
        search,
        f.limit
    )
    .fetch_all(db)
    .await
}

pub async fn get(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<Option<Transaction>> {
    sqlx::query_as!(
        Transaction,
        r#"SELECT id, category_id, kind, amount, merchant, note, occurred_at, source,
                  receipt_id, items, needs_check, created_at
           FROM transactions WHERE user_id = $1 AND id = $2"#,
        user_id,
        id
    )
    .fetch_optional(db)
    .await
}

pub struct Patch<'a> {
    pub category_id: Option<Option<Uuid>>,
    pub amount: Option<i64>,
    pub merchant: Option<&'a str>,
    pub note: Option<Option<&'a str>>,
    pub occurred_at: Option<DateTime<Utc>>,
    pub needs_check: Option<bool>,
}

pub async fn update(
    db: &PgPool,
    user_id: Uuid,
    id: Uuid,
    p: Patch<'_>,
) -> sqlx::Result<Option<Transaction>> {
    sqlx::query_as!(
        Transaction,
        r#"UPDATE transactions SET
             category_id = CASE WHEN $3 THEN $4 ELSE category_id END,
             amount = COALESCE($5, amount),
             merchant = COALESCE($6, merchant),
             note = CASE WHEN $7 THEN $8 ELSE note END,
             occurred_at = COALESCE($9, occurred_at),
             needs_check = COALESCE($10, needs_check),
             updated_at = now()
           WHERE user_id = $1 AND id = $2
           RETURNING id, category_id, kind, amount, merchant, note, occurred_at, source,
                     receipt_id, items, needs_check, created_at"#,
        user_id,
        id,
        p.category_id.is_some(),
        p.category_id.flatten(),
        p.amount,
        p.merchant,
        p.note.is_some(),
        p.note.flatten(),
        p.occurred_at,
        p.needs_check
    )
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let res = sqlx::query!(
        "DELETE FROM transactions WHERE user_id = $1 AND id = $2",
        user_id,
        id
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

#[derive(Debug, Clone)]
pub struct CategoryTotal {
    pub category_id: Option<Uuid>,
    pub amount: i64,
}

pub async fn expense_by_category(
    db: &PgPool,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> sqlx::Result<Vec<CategoryTotal>> {
    sqlx::query_as!(
        CategoryTotal,
        r#"SELECT category_id, SUM(amount)::bigint AS "amount!"
           FROM transactions
           WHERE user_id = $1 AND kind = 'expense' AND occurred_at >= $2 AND occurred_at < $3
           GROUP BY category_id"#,
        user_id,
        from,
        to
    )
    .fetch_all(db)
    .await
}

/// (income, expense) for the period.
pub async fn totals(
    db: &PgPool,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> sqlx::Result<(i64, i64)> {
    let r = sqlx::query!(
        r#"SELECT
             COALESCE(SUM(amount) FILTER (WHERE kind = 'income'), 0)::bigint AS "income!",
             COALESCE(SUM(amount) FILTER (WHERE kind = 'expense'), 0)::bigint AS "expense!"
           FROM transactions
           WHERE user_id = $1 AND occurred_at >= $2 AND occurred_at < $3"#,
        user_id,
        from,
        to
    )
    .fetch_one(db)
    .await?;
    Ok((r.income, r.expense))
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyTotal {
    pub day: NaiveDate,
    pub amount: i64,
}

pub async fn daily_expense(
    db: &PgPool,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> sqlx::Result<Vec<DailyTotal>> {
    sqlx::query_as!(
        DailyTotal,
        r#"SELECT (occurred_at AT TIME ZONE 'Asia/Jakarta')::date AS "day!",
                  SUM(amount)::bigint AS "amount!"
           FROM transactions
           WHERE user_id = $1 AND kind = 'expense' AND occurred_at >= $2 AND occurred_at < $3
           GROUP BY 1 ORDER BY 1"#,
        user_id,
        from,
        to
    )
    .fetch_all(db)
    .await
}

pub async fn count_needs_check(db: &PgPool, user_id: Uuid) -> sqlx::Result<i64> {
    let r = sqlx::query!(
        r#"SELECT COUNT(*) AS "n!" FROM transactions WHERE user_id = $1 AND needs_check"#,
        user_id
    )
    .fetch_one(db)
    .await?;
    Ok(r.n)
}
