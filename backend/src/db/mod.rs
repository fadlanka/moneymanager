//! Data access. Every function that reads user data takes `user_id` and filters on it.

pub mod budgets;
pub mod categories;
pub mod debts;
pub mod receipts;
pub mod transactions;
pub mod users;

/// Postgres unique-violation SQLSTATE.
pub fn is_unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}
