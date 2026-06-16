// Basit Stalwart JMAP client örneği (Rust)
// Gereken: reqwest, serde, serde_json

use reqwest::Client;
use serde_json::json;
use anyhow::Result;

pub struct JmapClient {
    pub base_url: String,
    pub api_token: String,
    pub client: Client,
}

impl JmapClient {
    pub fn new(base_url: &str, api_token: &str) -> Self {
        JmapClient {
            base_url: base_url.to_string(),
            api_token: api_token.to_string(),
            client: Client::new(),
        }
    }

    // JMAP Session discovery
    pub async fn get_session(&self) -> Result<serde_json::Value> {
        let url = format!("{}/.well-known/jmap", self.base_url);
        let resp = self.client.get(&url)
            .bearer_auth(&self.api_token)
            .send().await?;
        Ok(resp.json().await?)
    }

    // Inbox mesajlarını getir
    pub async fn get_inbox_messages(&self, account_id: &str) -> Result<serde_json::Value> {
        let url = format!("{}/jmap", self.base_url);
        let body = json!({
            "using": [
                "urn:ietf:params:jmap:core",
                "urn:ietf:params:jmap:mail"
            ],
            "methodCalls": [
                ["Email/query", {
                    "accountId": account_id,
                    "filter": { "inMailbox": "inbox" },
                    "limit": 20
                }, "a"]
            ]
        });
        let resp = self.client.post(&url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send().await?;
        Ok(resp.json().await?)
    }

    // Mesaj gönder
    pub async fn send_email(&self, account_id: &str, to: &str, subject: &str, body_text: &str) -> Result<serde_json::Value> {
        let url = format!("{}/jmap", self.base_url);
        let body = json!({
            "using": [
                "urn:ietf:params:jmap:core",
                "urn:ietf:params:jmap:mail"
            ],
            "methodCalls": [
                ["Email/send", {
                    "accountId": account_id,
                    "from": [{ "email": to }],
                    "subject": subject,
                    "textBody": body_text
                }, "b"]
            ]
        });
        let resp = self.client.post(&url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send().await?;
        Ok(resp.json().await?)
    }
}
