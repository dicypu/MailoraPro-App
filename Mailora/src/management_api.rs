// Management API: Tenant & Domain provisioning
// API Key ile otomasyon
use serde::{Serialize, Deserialize};
use axum::{Json, http::StatusCode};

#[derive(Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub domains: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub domains: Vec<String>,
}

pub async fn create_tenant(Json(req): Json<CreateTenantRequest>) -> Result<Json<Tenant>, StatusCode> {
    // DB kaydı, API Key kontrolü vs. burada
    Ok(Json(Tenant {
        id: "tenant-uuid".to_string(),
        name: req.name,
        domains: req.domains,
    }))
}
