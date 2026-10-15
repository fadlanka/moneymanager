use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Budget {
    pub category_id: Uuid,
    pub month: NaiveDate,
    pub limit_amount: i64,
}

pub async fn list(db: &PgPool, user_id: Uuid, month: NaiveDate) -> sqlx::Result<Vec<Budget>> {
    sqlx::query_as!(
        Budget,
        "SELECT category_id, month, limit_amount FROM budgets WHERE user_id = $1 AND month = $2",
        user_id,
        month
    )
    .fetch_all(db)
    .await
}

/// Latest month before `month` that has any budgets; used to carry limits forward.
pub async fn latest_before(
    db: &PgPool,
    user_id: Uuid,
    month: NaiveDate,
) -> sqlx::Result<Option<NaiveDate>> {
    let r = sqlx::query!(
        "SELECT MAX(month) AS month FROM budgets WHERE user_id = $1 AND month < $2",
        user_id,
        month
    )
    .fetch_one(db)
    .await?;
    Ok(r.month)
}

pub async fn upsert(
    conn: &mut PgConnection,
    user_id: Uuid,
    category_id: Uuid,
    month: NaiveDate,
    limit_amount: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO budgets (user_id, category_id, month, limit_amount)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, category_id, month) DO UPDATE SET limit_amount = EXCLUDED.limit_amount"#,
        user_id,
        category_id,
        month,
        limit_amount
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn delete(
    conn: &mut PgConnection,
    user_id: Uuid,
    category_id: Uuid,
    month: NaiveDate,
) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM budgets WHERE user_id = $1 AND category_id = $2 AND month = $3",
        user_id,
        category_id,
        month
    )
    .execute(conn)
    .await?;
    Ok(())
}
