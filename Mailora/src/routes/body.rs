use axum::{routing::get, Router};

pub async fn body_handler() -> &'static str {
    "Body endpoint"
}

pub fn create_routes() -> Router {
    Router::new().route("/", get(body_handler))
}