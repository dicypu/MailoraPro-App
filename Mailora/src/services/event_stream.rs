use axum::{response::IntoResponse, routing::Router};
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone)]
pub struct EventStream {}

#[allow(dead_code)]
impl EventStream {
    pub fn new() -> Self {
        EventStream {}
    }

    pub async fn handle_events(self: Arc<Self>) -> impl IntoResponse {
        axum::Json(serde_json::json!({"ok":true}))
    }
}

#[allow(dead_code)]
pub fn create_routes(event_stream: Arc<EventStream>) -> Router {
    Router::new().route(
        "/events",
        axum::routing::get({
            let es = event_stream.clone();
            move || {
                let es = es.clone();
                async move { es.handle_events().await }
            }
        }),
    )
}
