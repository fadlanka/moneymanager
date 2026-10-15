use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Category {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub kind: String,
    pub color: String,
    pub flexible: bool,
}

/// (slug, name, kind, color token, flexible). Colors map to design tokens in the app.
pub const DEFAULTS: &[(&str, &str, &str, &str, bool)] = &[
    ("food", "Food & drinks", "expense", "yellow", true),
    ("groceries", "Groceries", "expense", "green-soft", true),
    ("shopping", "Shopping", "expense", "red", true),
    ("transport", "Transport", "expense", "green", true),
    ("housing", "Housing", "expense", "blue", false),
    ("bills", "Bills & utilities", "expense", "blue-soft", false),
    ("subscriptions", "Subscriptions", "expense", "ink", true),
    ("health", "Health", "expense", "red-soft", false),
    (
        "entertainment",
        "Entertainment",
        "expense",
        "yellow-soft",
        true,
    ),
    ("debt", "Debt & installments", "expense", "grey", false),
    ("other", "Other", "expense", "grey", true),
    ("income", "Income", "income", "green-soft", false),
];

pub async fn seed_defaults(conn: &mut PgConnection, user_id: Uuid) -> sqlx::Result<()> {
    for (slug, name, kind, color, flexible) in DEFAULTS {
        sqlx::query!(
            r#"INSERT INTO categories (user_id, slug, name, kind, color, flexible)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (user_id, slug) DO NOTHING"#,
            user_id,
            slug,
            name,
            kind,
            color,
            flexible
        )
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub async fn list(db: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Category>> {
    sqlx::query_as!(
        Category,
        r#"SELECT id, slug, name, kind, color, flexible FROM categories
           WHERE user_id = $1 ORDER BY kind DESC, created_at, name"#,
        user_id
    )
    .fetch_all(db)
    .await
}

pub async fn get(db: &PgPool, user_id: Uuid, id: Uuid) -> sqlx::Result<Option<Category>> {
    sqlx::query_as!(
        Category,
        "SELECT id, slug, name, kind, color, flexible FROM categories WHERE user_id = $1 AND id = $2",
        user_id,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn find_by_slug(
    db: &PgPool,
    user_id: Uuid,
    slug: &str,
) -> sqlx::Result<Option<Category>> {
    sqlx::query_as!(
        Category,
        "SELECT id, slug, name, kind, color, flexible FROM categories WHERE user_id = $1 AND slug = $2",
        user_id,
        slug
    )
    .fetch_optional(db)
    .await
}

pub async fn create(
    db: &PgPool,
    user_id: Uuid,
    slug: &str,
    name: &str,
    kind: &str,
    color: &str,
    flexible: bool,
) -> sqlx::Result<Category> {
    sqlx::query_as!(
        Category,
        r#"INSERT INTO categories (user_id, slug, name, kind, color, flexible)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, slug, name, kind, color, flexible"#,
        user_id,
        slug,
        name,
        kind,
        color,
        flexible
    )
    .fetch_one(db)
    .await
}
