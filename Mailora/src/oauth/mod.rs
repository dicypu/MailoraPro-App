use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>,
    pub token_type: String,
}

pub struct OAuthManager {
    // Store pending auth requests: state -> (provider, account_id, pkce_verifier)
    pending_auths: Arc<RwLock<HashMap<String, (String, String, PkceCodeVerifier)>>>,
}

impl OAuthManager {
    pub fn new() -> Self {
        Self {
            pending_auths: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get OAuth2 config for a provider
    pub fn get_provider_config(provider: &str) -> Result<OAuthConfig, String> {
        match provider {
            "gmail" => Ok(OAuthConfig {
                client_id: std::env::var("GOOGLE_CLIENT_ID")
                    .unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_ID".to_string()),
                client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                    .unwrap_or_else(|_| "YOUR_GOOGLE_CLIENT_SECRET".to_string()),
                auth_url: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_url: "https://oauth2.googleapis.com/token".to_string(),
                redirect_uri: "http://localhost:3030/oauth/callback".to_string(),
                scopes: vec![
                    "https://mail.google.com/".to_string(),
                    "https://www.googleapis.com/auth/userinfo.email".to_string(),
                ],
            }),
            "outlook" => Ok(OAuthConfig {
                client_id: std::env::var("MICROSOFT_CLIENT_ID")
                    .unwrap_or_else(|_| "YOUR_MICROSOFT_CLIENT_ID".to_string()),
                client_secret: std::env::var("MICROSOFT_CLIENT_SECRET")
                    .unwrap_or_else(|_| "YOUR_MICROSOFT_CLIENT_SECRET".to_string()),
                auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize"
                    .to_string(),
                token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
                redirect_uri: "http://localhost:3030/oauth/callback".to_string(),
                scopes: vec![
                    "https://outlook.office365.com/IMAP.AccessAsUser.All".to_string(),
                    "https://outlook.office365.com/SMTP.Send".to_string(),
                    "offline_access".to_string(),
                ],
            }),
            "yahoo" => Ok(OAuthConfig {
                client_id: std::env::var("YAHOO_CLIENT_ID")
                    .unwrap_or_else(|_| "YOUR_YAHOO_CLIENT_ID".to_string()),
                client_secret: std::env::var("YAHOO_CLIENT_SECRET")
                    .unwrap_or_else(|_| "YOUR_YAHOO_CLIENT_SECRET".to_string()),
                auth_url: "https://api.login.yahoo.com/oauth2/request_auth".to_string(),
                token_url: "https://api.login.yahoo.com/oauth2/get_token".to_string(),
                redirect_uri: "http://localhost:3030/oauth/callback".to_string(),
                scopes: vec!["mail-w".to_string()],
            }),
            _ => Err(format!("OAuth not supported for provider: {}", provider)),
        }
    }

    /// Generate authorization URL for OAuth flow
    pub async fn start_auth_flow(
        &self,
        provider: &str,
        account_id: &str,
    ) -> Result<String, String> {
        let config = Self::get_provider_config(provider)?;

        let client = BasicClient::new(
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret)),
            AuthUrl::new(config.auth_url).map_err(|e| e.to_string())?,
            Some(TokenUrl::new(config.token_url).map_err(|e| e.to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri).map_err(|e| e.to_string())?);

        // Generate PKCE challenge (more secure than plain state)
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Build authorization URL
        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        for scope in config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope));
        }

        let (auth_url, csrf_state) = auth_request.url();

        // Store state and PKCE verifier for validation
        self.pending_auths.write().await.insert(
            csrf_state.secret().clone(),
            (provider.to_string(), account_id.to_string(), pkce_verifier),
        );

        Ok(auth_url.to_string())
    }

    /// Handle OAuth callback and exchange code for tokens
    pub async fn handle_callback(
        &self,
        code: String,
        state: String,
    ) -> Result<(String, String, OAuthTokens), String> {
        // Validate state and get PKCE verifier
        let (provider, account_id, pkce_verifier) = self
            .pending_auths
            .write()
            .await
            .remove(&state)
            .ok_or("Invalid or expired state")?;

        let config = Self::get_provider_config(&provider)?;

        let client = BasicClient::new(
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret)),
            AuthUrl::new(config.auth_url).map_err(|e| e.to_string())?,
            Some(TokenUrl::new(config.token_url).map_err(|e| e.to_string())?),
        )
        .set_redirect_uri(RedirectUrl::new(config.redirect_uri).map_err(|e| e.to_string())?);

        // Exchange code for token WITH PKCE VERIFIER
        let token_result = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                eprintln!("❌ Token exchange error: {:?}", e);
                format!("Token exchange failed: {:?}", e)
            })?;

        let tokens = OAuthTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result.refresh_token().map(|t| t.secret().clone()),
            expires_at: token_result
                .expires_in()
                .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64),
            token_type: "Bearer".to_string(),
        };

        Ok((provider, account_id, tokens))
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(provider: &str, refresh_token: &str) -> Result<OAuthTokens, String> {
        let config = Self::get_provider_config(provider)?;

        let client = BasicClient::new(
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret)),
            AuthUrl::new(config.auth_url).map_err(|e| e.to_string())?,
            Some(TokenUrl::new(config.token_url).map_err(|e| e.to_string())?),
        );

        let token_result = client
            .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| format!("Token refresh failed: {}", e))?;

        Ok(OAuthTokens {
            access_token: token_result.access_token().secret().clone(),
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().clone())
                .or_else(|| Some(refresh_token.to_string())), // Keep old refresh token if not returned
            expires_at: token_result
                .expires_in()
                .map(|d| chrono::Utc::now().timestamp() + d.as_secs() as i64),
            token_type: "Bearer".to_string(),
        })
    }

    /// Get valid access token (auto-refresh if expired)
    pub async fn get_valid_token(
        provider: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<i64>,
    ) -> Result<String, String> {
        // Check if token is expired or will expire soon (5 min buffer)
        let is_expired = expires_at
            .map(|exp| exp < chrono::Utc::now().timestamp() + 300)
            .unwrap_or(false);

        if is_expired {
            if let Some(refresh) = refresh_token {
                let new_tokens = Self::refresh_token(provider, refresh).await?;
                Ok(new_tokens.access_token)
            } else {
                Err("Token expired and no refresh token available".to_string())
            }
        } else {
            Ok(access_token.to_string())
        }
    }
}

#[allow(dead_code)]
/// XOAUTH2 authentication string for IMAP/SMTP
pub fn generate_xoauth2_string(email: &str, access_token: &str) -> String {
    format!("user={}\x01auth=Bearer {}\x01\x01", email, access_token)
}
