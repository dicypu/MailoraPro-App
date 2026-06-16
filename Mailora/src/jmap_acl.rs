// JMAP ACL/Sharing örneği
use serde_json::json;
use anyhow::Result;
use reqwest::Client;

pub async fn set_mailbox_acl(api_url: &str, api_token: &str, mailbox_id: &str, user_id: &str, rights: Vec<&str>) -> Result<serde_json::Value> {
    let body = json!({
        "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail", "urn:ietf:params:jmap:sharing"],
        "methodCalls": [
            ["Mailbox/setRights", {
                "mailboxId": mailbox_id,
                "userId": user_id,
                "rights": rights
            }, "acl1"]
        ]
    });
    let resp = Client::new().post(api_url)
        .bearer_auth(api_token)
        .json(&body)
        .send().await?;
    Ok(resp.json().await?)
}
