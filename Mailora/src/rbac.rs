use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub role: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok());

        if let Some(token) = auth_header {
            // Simple token format: "id:role" (MVP)
            let parts: Vec<&str> = token.split(':').collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[0].parse::<i64>() {
                    return Ok(AuthUser {
                        id,
                        role: parts[1].to_string(),
                    });
                }
            }
        }

        Err((StatusCode::UNAUTHORIZED, "Missing or invalid token"))
    }
}

pub struct AdminUser(pub AuthUser);

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await?;
        if auth_user.role == "Admin" {
            Ok(AdminUser(auth_user))
        } else {
            Err((StatusCode::FORBIDDEN, "Admin rights required"))
        }
    }
}

pub async fn log_event(
    pool: &sqlx::SqlitePool,
    user_id: Option<i64>,
    account_id: Option<&str>,
    action: &str,
    details: &str,
) {
    let _ = sqlx::query("INSERT INTO event_logs (user_id, account_id, action, details) VALUES (?, ?, ?, ?)")
        .bind(user_id)
        .bind(account_id)
        .bind(action)
        .bind(details)
        .execute(pool)
        .await;
}
