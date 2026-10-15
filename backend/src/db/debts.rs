use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Debt {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub creditor: Option<String>,
    pub principal: i64,
    pub balance: i64,
    pub monthly_payment: i64,
    pub interest_rate_bp: i32,
    pub due_day: Option<i32>,
    pub created_at: DateTime<Utc>,
}

pub struct NewDebt<'a> {
    pub name: &'a str,
    pub kind: &'a str,
    pub creditor: Option<&'a str>,
    pub principal: i64,
    pub balance: i64,
    pub monthly_payment: i64,
    pub interest_rate_bp: i32,
    pub due_day: Option<i32>,
}

pub async fn list(db: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Debt>> {
    sqlx::query_as!(
        Debt,
        r#"SELECT id, name, kind, creditor, principal, balance, monthly_payment, interest_rate_bp, due_day, created_at
           FROM debts WHERE user_id = $1 ORDER BY balance = 0, due_day NULLS LAST, created_at"#,
        user_id
    )
    .fetch_all(db)
    .await
}

pub async fn get_for_update(
    conn: &mut PgConnection,
    user_id: Uuid,
    id: Uuid,
) -> sqlx::Result<Option<Debt>> {
    sqlx::query_as!(
        Debt,
        r#"SELECT id, name, kind, creditor, principal, balance, monthly_payment, interest_rate_bp, due_day, created_at
           FROM debts WHERE user_id = $1 AND id = $2 FOR UPDATE"#,
        user_id,
        id
    )
    .fetch_optional(conn)
    .await
}

pub async fn create(db: &PgPool, user_id: Uuid, d: NewDebt<'_>) -> sqlx::Result<Debt> {
    sqlx::query_as!(
        Debt,
        r#"INSERT INTO debts (user_id, name, kind, creditor, principal, balance, monthly_payment, interest_rate_bp, due_day)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING id, name, kind, creditor, principal, balance, monthly_payment, interest_rate_bp, due_day, created_at"#,
        user_id,
        d.name,
        d.kind,
        d.creditor,
        d.principal,
        d.balance,
        d.monthly_payment,
        d.interest_rate_bp,
        d.due_day
    )
    .fetch_one(db)
    .await
}

pub async fn update(
    db: &PgPool,
    user_id: Uuid,
    id: Uuid,
    d: NewDebt<'_>,
) -> sqlx::Result<Option<Debt>> {
    sqlx::query_as!(
        Debt,
        r#"UPDATE debts SET name = $3, kind = $4, creditor = $5, principal = $6, balance = $7,
             monthly_payment = $8, interest_rate_bp = $9, due_day = $10, updated_at = now()
           WHERE user_id = $1 AND id = $2
           RETURNING id, name, kind, creditor, principal, balance, monthly_payment, interest_rate_bp, due_day, created_at"#,
        user_id,
        id,
        d.name,
        d.kind,
        d.creditor,
        d.principal,
        d.balance,
        d.monthly_payment,
        d.interest_rate_bp,
        d.due_day
    )
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<bool> {
    let res = sqlx::query!(
        "DELETE FROM debts WHERE user_id = $1 AND id = $2",
        user_id,
        id
    )
    .execute(db)
    .await?;
    Ok(res.rows_affected() == 1)
}

pub async fn set_balance(conn: &mut PgConnection, id: Uuid, balance: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE debts SET balance = $2, updated_at = now() WHERE id = $1",
        id,
        balance
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn insert_payment(
    conn: &mut PgConnection,
    debt_id: Uuid,
    user_id: Uuid,
    amount: i64,
    paid_at: DateTime<Utc>,
    transaction_id: Option<Uuid>,
) -> sqlx::Result<Uuid> {
    let r = sqlx::query!(
        r#"INSERT INTO debt_payments (debt_id, user_id, amount, paid_at, transaction_id)
           VALUES ($1, $2, $3, $4, $5) RETURNING id"#,
        debt_id,
        user_id,
        amount,
        paid_at,
        transaction_id
    )
    .fetch_one(conn)
    .await?;
    Ok(r.id)
}

/// Total paid per debt within [from, to).
pub async fn paid_between(
    db: &PgPool,
    user_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> sqlx::Result<Vec<(Uuid, i64)>> {
    let rows = sqlx::query!(
        r#"SELECT debt_id, SUM(amount)::bigint AS "paid!"
           FROM debt_payments
           WHERE user_id = $1 AND paid_at >= $2 AND paid_at < $3
           GROUP BY debt_id"#,
        user_id,
        from,
        to
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| (r.debt_id, r.paid)).collect())
}
