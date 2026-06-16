use crate::models::user::{CreateUserReq, User};
use anyhow::Result;
use sqlx::SqlitePool;
use bcrypt::{hash, verify, DEFAULT_COST};

pub async fn register_user(pool: &SqlitePool, req: CreateUserReq) -> Result<User> {
    // Check if any users exist. If not, make this one Admin.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    
    let role = if count == 0 { "Admin" } else { "Member" };
    let hash = hash(req.password, DEFAULT_COST)?;

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO users (username, password_hash, role) VALUES (?, ?, ?) RETURNING id"
    )
    .bind(&req.username)
    .bind(&hash)
    .bind(role)
    .fetch_one(pool)
    .await?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;

    Ok(user)
}

pub async fn verify_user(pool: &SqlitePool, username: &str, password: &str) -> Result<Option<User>> {
    let user_opt = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?;

    if let Some(user) = user_opt {
        if verify(password, &user.password_hash)? {
            return Ok(Some(user));
        }
    }
    Ok(None)
}
