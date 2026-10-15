use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub monthly_income: i64,
    pub created_at: DateTime<Utc>,
}

pub async fn create(
    conn: &mut PgConnection,
    email: &str,
    name: &str,
    password_hash: &str,
) -> sqlx::Result<User> {
    sqlx::query_as!(
        User,
        r#"INSERT INTO users (email, name, password_hash)
           VALUES ($1, $2, $3)
           RETURNING id, email, name, monthly_income, created_at"#,
        email,
        name,
        password_hash
    )
    .fetch_one(conn)
    .await
}

pub async fn find_by_email(db: &PgPool, email: &str) -> sqlx::Result<Option<(User, String)>> {
    let row = sqlx::query!(
        r#"SELECT id, email, name, monthly_income, created_at, password_hash
           FROM users WHERE email = $1"#,
        email
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| {
        (
            User {
                id: r.id,
                email: r.email,
                name: r.name,
                monthly_income: r.monthly_income,
                created_at: r.created_at,
            },
            r.password_hash,
        )
    }))
}

pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<User> {
    sqlx::query_as!(
        User,
        "SELECT id, email, name, monthly_income, created_at FROM users WHERE id = $1",
        id
    )
    .fetch_one(db)
    .await
}

pub async fn update(
    db: &PgPool,
    id: Uuid,
    name: Option<&str>,
    monthly_income: Option<i64>,
) -> sqlx::Result<User> {
    sqlx::query_as!(
        User,
        r#"UPDATE users SET
             name = COALESCE($2, name),
             monthly_income = COALESCE($3, monthly_income)
           WHERE id = $1
           RETURNING id, email, name, monthly_income, created_at"#,
        id,
        name,
        monthly_income
    )
    .fetch_one(db)
    .await
}
