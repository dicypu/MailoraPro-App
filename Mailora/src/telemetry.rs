// Telemetry & Audit: Event akışı
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub user_id: String,
    pub timestamp: u64,
    pub details: String,
}

pub fn log_event(event: Event) {
    // Prometheus/Loki entegrasyonu burada
    println!("EVENT: {:?}", event);
}
