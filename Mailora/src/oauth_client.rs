// OAuth 2.0/OIDC discovery & token alma
use reqwest::Client;
use serde_json::Value;
use anyhow::Result;

pub async fn discover_oauth_meta(base_url: &str) -> Result<Value> {
    let url = format!("{}/.well-known/oauth-authorization-server", base_url);
    let resp = Client::new().get(&url).send().await?;
    Ok(resp.json().await?)
}

pub async fn get_token(token_endpoint: &str, client_id: &str, code: &str, redirect_uri: &str) -> Result<Value> {
    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", redirect_uri),
    ];
    let resp = Client::new().post(token_endpoint)
        .form(&params)
        .send().await?;
    Ok(resp.json().await?)
}
