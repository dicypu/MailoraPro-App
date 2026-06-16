use axum::http::StatusCode as AxumStatus;
use axum::{extract::State, http::HeaderMap, response::IntoResponse};

#[derive(Clone)]
#[allow(dead_code)]
pub struct JmapProxyState {
    pub http: reqwest::Client,
    pub jmap_base: String,
}

#[allow(dead_code)]
pub async fn proxy_jmap(
    State(st): State<JmapProxyState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    let url = format!("{}/jmap", st.jmap_base.trim_end_matches('/'));
    let mut req = st
        .http
        .post(url)
        .body(body)
        .header("Content-Type", "application/json");
    if let Some(auth) = headers.get("Authorization") {
        if let Ok(s) = auth.to_str() {
            req = req.header("Authorization", s);
        }
    }
    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(text) => (
                    AxumStatus::from_u16(status.as_u16()).unwrap_or(AxumStatus::BAD_GATEWAY),
                    text,
                )
                    .into_response(),
                Err(_) => (AxumStatus::BAD_GATEWAY, "upstream read error").into_response(),
            }
        }
        Err(_) => (AxumStatus::BAD_GATEWAY, "upstream error").into_response(),
    }
}

#[allow(dead_code)]
pub async fn proxy_well_known(State(st): State<JmapProxyState>) -> impl IntoResponse {
    let url = format!("{}/.well-known/jmap", st.jmap_base.trim_end_matches('/'));
    match st.http.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(text) => (
                    AxumStatus::from_u16(status.as_u16()).unwrap_or(AxumStatus::BAD_GATEWAY),
                    text,
                )
                    .into_response(),
                Err(_) => (AxumStatus::BAD_GATEWAY, "upstream read error").into_response(),
            }
        }
        Err(_) => (AxumStatus::BAD_GATEWAY, "upstream error").into_response(),
    }
}

#[allow(dead_code)]
pub async fn proxy_session(State(st): State<JmapProxyState>) -> impl IntoResponse {
    let url = format!("{}/jmap/session", st.jmap_base.trim_end_matches('/'));
    match st.http.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.text().await {
                Ok(text) => (
                    AxumStatus::from_u16(status.as_u16()).unwrap_or(AxumStatus::BAD_GATEWAY),
                    text,
                )
                    .into_response(),
                Err(_) => (AxumStatus::BAD_GATEWAY, "upstream read error").into_response(),
            }
        }
        Err(_) => (AxumStatus::BAD_GATEWAY, "upstream error").into_response(),
    }
}
