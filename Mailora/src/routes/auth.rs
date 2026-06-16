use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use crate::models::user::{CreateUserReq, LoginUserReq, AuthResponse};
use crate::services::auth_service;

async fn register(
    State(pool): State<sqlx::SqlitePool>,
    Json(req): Json<CreateUserReq>,
) -> impl IntoResponse {
    match auth_service::register_user(&pool, req).await {
        Ok(user) => (StatusCode::CREATED, Json(serde_json::json!({
            "ok": true,
            "username": user.username,
            "role": user.role
        }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "ok": false,
            "error": e.to_string()
        }))).into_response(),
    }
}

async fn login(
    State(pool): State<sqlx::SqlitePool>,
    Json(req): Json<LoginUserReq>,
) -> impl IntoResponse {
    match auth_service::verify_user(&pool, &req.username, &req.password).await {
        Ok(Some(user)) => {
            // For MVP, simple token: "id:role" (in real world use JWT)
            let token = format!("{}:{}", user.id, user.role);
            (StatusCode::OK, Json(AuthResponse {
                token,
                username: user.username,
                role: user.role,
            })).into_response()
        }
        Ok(None) => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "ok": false,
            "error": "Invalid username or password"
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "ok": false,
            "error": e.to_string()
        }))).into_response(),
    }
}

pub fn router() -> Router<sqlx::SqlitePool> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
}
