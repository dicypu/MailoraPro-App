use axum::{
    Json,
    Router,
    routing::post,
    response::IntoResponse,
    http::StatusCode,
};
use crate::services::discovery_service::{DiscoveryService, DiscoveryConfig};
use serde_json::json;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/discover", post(discover))
}

async fn discover(
    Json(payload): Json<DiscoveryConfig>,
) -> impl IntoResponse {
    let service = DiscoveryService::new();
    
    match service.discover(&payload.email).await {
        Ok(result) => (StatusCode::OK, Json(result)).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND, 
            Json(json!({ "error": e.to_string() }))
        ).into_response()
    }
}
